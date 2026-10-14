import { useState } from "react";
import { Act } from "./Act";

export function RecordCopy({ name, value }: { name: string; value: string }) {
  const [state, setState] = useState<"ready" | "copying" | "copied" | "failed">("ready");
  const copy = async () => {
    if (state === "copying") return;
    setState("copying");
    try {
      if (!navigator.clipboard) throw new Error("Clipboard unavailable");
      await navigator.clipboard.writeText(value);
      setState("copied");
    } catch {
      setState("failed");
    }
  };
  return (
    <span className="record-copy" title={value}>
      <span>{name}</span>
      <Act
        symbol={state === "copied" ? "approve" : "copy"}
        name={"Copy " + name}
        word={state === "copied" ? "Copied" : "Copy"}
        hint={value}
        disabled={state === "copying"}
        onClick={() => void copy()}
      />
      {state === "failed" ? (
        <span role="alert">
          Copy could not be confirmed. Select the identifier here: <code>{value}</code>
        </span>
      ) : null}
    </span>
  );
}
