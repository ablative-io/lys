"""Measure Claude Code's instruction load order against a local listener.

Prints orders, counts and hashes only; request bodies stay in the run
directories under this scratch tree and are never printed.
"""
import hashlib, json, os, shutil, subprocess, sys, threading, time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

S = os.path.dirname(os.path.abspath(__file__))
H = os.path.join(S, "home")
CFG = os.path.join(S, "config")
ANC = os.path.join(S, "anc.dot")
W = os.path.join(ANC, "w_us")
INSTR = os.path.join(S, "instr.md")
MCP = os.path.join(S, "mcp.json")

def slug_new(p): return "".join(c if c.isascii() and c.isalnum() else "-" for c in p)
def slug_old(p): return p.replace("/", "-")

MEM_NEW = os.path.join(CFG, "projects", slug_new(W), "memory", "MEMORY.md")
MEM_OLD = os.path.join(CFG, "projects", slug_old(W), "memory", "MEMORY.md")

FILES = [
    ("user_claude_md", os.path.join(CFG, "CLAUDE.md"), "MARKUSERCLAUDEMDZQ"),
    ("appended_instructions", INSTR, "MARKAPPENDEDINSTRZQ"),
    ("mcp_config", MCP, "markmcpserverzq"),
    ("claude_md_chain ancestor CLAUDE.md", os.path.join(ANC, "CLAUDE.md"), "MARKANCESTORCLAUDEMDZQ"),
    ("claude_md_chain W/CLAUDE.md", os.path.join(W, "CLAUDE.md"), "MARKWCLAUDEMDZQ"),
    ("claude_md_chain W/.claude/CLAUDE.md", os.path.join(W, ".claude", "CLAUDE.md"), "MARKWDOTCLAUDEZQ"),
    ("claude_md_chain W/CLAUDE.local.md", os.path.join(W, "CLAUDE.local.md"), "MARKWLOCALZQ"),
    ("memory_index new slug", MEM_NEW, "MARKMEMORYNEWSLUGZQ"),
    ("memory_index old slug", MEM_OLD, "MARKMEMORYOLDSLUGZQ"),
]

def write_fixtures():
    for name, path, marker in FILES:
        os.makedirs(os.path.dirname(path), exist_ok=True)
        if path == MCP:
            body = json.dumps({"mcpServers": {marker: {"command": "/usr/bin/true", "args": []}}}) + "\n"
        else:
            body = f"# fixture {name}\n\nThe marker for this position is {marker}.\n"
        with open(path, "w") as f:
            f.write(body)

class Handler(BaseHTTPRequestHandler):
    run_dir = None
    count = 0
    def log_message(self, *a): pass
    def do_POST(self):
        n = int(self.headers.get("content-length", "0"))
        body = self.rfile.read(n)
        Handler.count += 1
        with open(os.path.join(Handler.run_dir, f"req-{Handler.count}.json"), "wb") as f:
            f.write(body)
        with open(os.path.join(Handler.run_dir, f"req-{Handler.count}.path"), "w") as f:
            f.write(self.path + "\n")
        sse = [
            ("message_start", {"type": "message_start", "message": {"id": "msg_fixture", "type": "message", "role": "assistant", "model": "claude-fixture", "content": [], "stop_reason": None, "stop_sequence": None, "usage": {"input_tokens": 1, "output_tokens": 1}}}),
            ("content_block_start", {"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}),
            ("content_block_delta", {"type": "content_block_delta", "index": 0, "delta": {"type": "text_delta", "text": "ok"}}),
            ("content_block_stop", {"type": "content_block_stop", "index": 0}),
            ("message_delta", {"type": "message_delta", "delta": {"stop_reason": "end_turn", "stop_sequence": None}, "usage": {"output_tokens": 1}}),
            ("message_stop", {"type": "message_stop"}),
        ]
        out = "".join(f"event: {e}\ndata: {json.dumps(d)}\n\n" for e, d in sse).encode()
        self.send_response(200)
        self.send_header("content-type", "text/event-stream")
        self.send_header("content-length", str(len(out)))
        self.end_headers()
        self.wfile.write(out)

def texts_of(body):
    out = []
    sys_ = body.get("system", [])
    if isinstance(sys_, str): out.append(("system", sys_))
    else:
        for i, b in enumerate(sys_):
            if isinstance(b, dict) and b.get("type") == "text": out.append((f"system[{i}]", b.get("text", "")))
    for i, m in enumerate(body.get("messages", [])):
        c = m.get("content")
        if isinstance(c, str): out.append((f"messages[{i}]", c))
        else:
            for j, b in enumerate(c or []):
                if isinstance(b, dict) and b.get("type") == "text": out.append((f"messages[{i}][{j}]", b.get("text", "")))
    tools = json.dumps(body.get("tools", []))
    out.append(("tools", tools))
    return out

def analyse(run_dir):
    reqs = sorted(f for f in os.listdir(run_dir) if f.startswith("req-") and f.endswith(".json"))
    result = []
    for r in reqs:
        path = open(os.path.join(run_dir, r.replace(".json", ".path"))).read().strip()
        try: body = json.loads(open(os.path.join(run_dir, r), "rb").read())
        except Exception as e: result.append({"request": r, "path": path, "parse": str(e)}); continue
        parts = texts_of(body)
        joined = ""
        found = []
        for where, t in parts:
            for name, _, marker in FILES:
                if marker in t and not any(f[0] == name for f in found):
                    found.append((name, where, len(joined) + t.index(marker)))
            joined += t + "\n"
        found.sort(key=lambda x: x[2])
        result.append({"request": r, "path": path, "model": body.get("model"), "system_blocks": len(body.get("system", [])) if not isinstance(body.get("system"), str) else 1,
                       "messages": len(body.get("messages", [])), "order": [(n, w) for n, w, _ in found],
                       "absent": [n for n, _, _ in FILES if n not in [f[0] for f in found]]})
    return result

def main():
    port = 47831
    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    summary = []
    for run in (1, 2, 3):
        run_dir = os.path.join(S, f"run{run}")
        os.makedirs(run_dir, exist_ok=True)
        Handler.run_dir = run_dir; Handler.count = 0
        shutil.rmtree(H, ignore_errors=True); os.makedirs(H)
        shutil.rmtree(CFG, ignore_errors=True)
        write_fixtures()
        time.sleep(1.2)
        env = {"PATH": os.environ["PATH"], "HOME": H, "CLAUDE_CONFIG_DIR": CFG, "TMPDIR": os.environ.get("TMPDIR", "/tmp"),
               "ANTHROPIC_BASE_URL": f"http://127.0.0.1:{port}", "ANTHROPIC_API_KEY": "sk-ant-fixture-not-a-key",
               "DISABLE_TELEMETRY": "1", "DISABLE_ERROR_REPORTING": "1", "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1", "TERM": "dumb", "LANG": "en_US.UTF-8"}
        cmd = ["claude", "-p", "Reply with the single word ok.", "--output-format", "json", "--max-turns", "1",
               "--mcp-config", MCP, "--strict-mcp-config", "--append-system-prompt-file", INSTR]
        t0 = time.time()
        try:
            p = subprocess.run(cmd, cwd=W, env=env, capture_output=True, timeout=120)
            exit_code, timed_out = p.returncode, False
            open(os.path.join(run_dir, "stdout.txt"), "wb").write(p.stdout)
            open(os.path.join(run_dir, "stderr.txt"), "wb").write(p.stderr)
        except subprocess.TimeoutExpired as e:
            exit_code, timed_out = None, True
            open(os.path.join(run_dir, "stdout.txt"), "wb").write(e.stdout or b"")
            open(os.path.join(run_dir, "stderr.txt"), "wb").write(e.stderr or b"")
        elapsed = round(time.time() - t0, 1)
        atimes = []
        for name, path, _ in FILES:
            st = os.stat(path)
            atimes.append((name, st.st_atime_ns, st.st_atime_ns > st.st_mtime_ns))
        read = sorted([a for a in atimes if a[2]], key=lambda a: a[1])
        unread = [a[0] for a in atimes if not a[2]]
        summary.append({"run": run, "exit": exit_code, "timed_out": timed_out, "seconds": elapsed, "requests": Handler.count,
                        "atime_order": [n for n, _, _ in read], "atime_ns": [(n, t) for n, t, _ in read], "never_read": unread,
                        "requests_analysed": analyse(run_dir),
                        "stderr_bytes": os.path.getsize(os.path.join(run_dir, "stderr.txt")), "stdout_bytes": os.path.getsize(os.path.join(run_dir, "stdout.txt"))})
    server.shutdown()
    print(json.dumps(summary, indent=1))
    print("W:", W); print("slug_new:", slug_new(W)); print("slug_old:", slug_old(W))
    for name, path, _ in FILES:
        b = open(path, "rb").read()
        print(name, len(b), hashlib.sha256(b).hexdigest())

main()
