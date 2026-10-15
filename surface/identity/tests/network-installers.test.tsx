/** Installer downloads keep the server's file name and use the existing connection state. */
import { act, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { request } from '../src/api';
import { ConnectRow } from '../src/features/network/JoinCode';
import type { Connecting } from '../src/features/network/JoinCode';

const machine = 'op-' + 'a'.repeat(32);
const initial: Connecting = { machine, name: 'Studio computer', given: null, failure: '', at: 'row' };
let root: ReturnType<typeof createRoot> | null = null;
let downloads: { name: string; url: string; attached: boolean }[] = [];
const created = vi.fn(() => 'blob:lys-installer');
const revoked = vi.fn();

beforeEach(() => {
  sessionStorage.clear();
  downloads = []; created.mockClear(); revoked.mockClear();
  const NativeURL = URL;
  vi.stubGlobal('URL', class extends NativeURL {
    static createObjectURL = created;
    static revokeObjectURL = revoked;
  });
  vi.spyOn(HTMLAnchorElement.prototype, 'click').mockImplementation(function (this: HTMLAnchorElement) {
    downloads.push({ name: this.download, url: this.href, attached: this.isConnected });
  });
});
afterEach(() => { if (root) act(() => root?.unmount()); root = null; });

async function draw(answer: () => Response | Promise<Response>) {
  const exchanges = vi.fn(async () => answer());
  vi.stubGlobal('fetch', exchanges);
  const updates: Connecting[] = [];
  function Connected() {
    const [connecting, connect] = useState(initial);
    return <table><tbody><ConnectRow connecting={connecting} connect={(next) => { updates.push(next); connect(next); }} done={() => {}} width={4} /></tbody></table>;
  }
  const host = document.createElement('div'); document.body.append(host);
  root = createRoot(host);
  await act(async () => root?.render(<Connected />));
  return { host, exchanges, updates };
}

async function choose(host: HTMLElement, kind: string) {
  const select = host.querySelector<HTMLSelectElement>('select[aria-label="What kind of computer is Studio computer?"]');
  if (!select) throw new Error('missing installer choice');
  await act(async () => { select.value = kind; select.dispatchEvent(new Event('change', { bubbles: true })); });
}
async function download(host: HTMLElement) {
  const button = host.querySelector<HTMLButtonElement>('button[aria-label="Download the installer for Studio computer"]');
  if (!button) throw new Error('missing installer download');
  await act(async () => button.click());
}
const file = (name: string) => new Response('installer fixture', { headers: { 'content-disposition': 'attachment; filename="' + name + '"', 'content-type': 'application/octet-stream' } });

it('offers the three computer kinds without fetching releases or a connection code', async () => {
  const { host, exchanges } = await draw(() => file('lys-runner.pkg'));
  expect([...host.querySelectorAll('option')].map((option) => [option.value, option.textContent])).toEqual([
    ['', 'Choose its kind'], ['macos-pkg', 'Mac'], ['deb', 'Linux: Debian or Ubuntu'], ['arch', 'Linux: Arch'],
  ]);
  expect(host.querySelector<HTMLButtonElement>('button[aria-label="Download the installer for Studio computer"]')?.disabled).toBe(true);
  expect(host.querySelector('button[aria-label="Download the installer for Studio computer"]')?.textContent).toBe('Download the installer for Studio computer');
  expect(host.querySelector('details[data-connect="terminal"]')?.hasAttribute('open')).toBe(false);
  expect(host.querySelector('[data-join="command"]')).toBeNull();
  expect(host.textContent).toContain('This computer appears here as Up once it connects.');
  expect(exchanges).not.toHaveBeenCalled();
});

it.each([
  ['macos-pkg', 'lys-studio.pkg', 'Open the file. If your Mac says it cannot check the developer or that the file was not opened, open System Settings, choose Privacy & Security, scroll down to the message about this installer and choose Open Anyway, then open the file again.'],
  ['deb', 'lys-studio_amd64.deb', 'Open the file and choose Install.'],
  ['arch', 'lys-studio-1-x86_64.pkg.tar.zst', 'sudo pacman -U lys-studio-1-x86_64.pkg.tar.zst'],
])('downloads %s as a blob with the server file name and its instructions', async (kind, name, words) => {
  const { host, exchanges, updates } = await draw(() => file(name));
  await choose(host, kind);
  await download(host);
  expect(exchanges).toHaveBeenCalledExactlyOnceWith('/api/network/machines/' + machine + '/installer/' + kind,
    expect.objectContaining({ method: 'POST', credentials: 'same-origin', body: '{}', headers: { accept: 'application/octet-stream', 'content-type': 'application/json' } }));
  expect(downloads).toEqual([{ name, url: 'blob:lys-installer', attached: true }]);
  expect(created).toHaveBeenCalledTimes(1);
  const saved = (created.mock.calls[0] as unknown[])[0] as Blob;
  expect(saved.size).toBe(17);
  expect(await saved.text()).toBe('installer fixture');
  expect(host.querySelector('a[download]')).toBeNull();
  expect(host.textContent).toContain(words);
  expect(host.textContent).toContain('Your browser was asked to save ' + name + '.');
  expect(updates.at(-1)).toEqual({ ...initial, given: { kind: 'installer', platform: kind, filename: name } });
  expect(sessionStorage.length).toBe(0);
  act(() => root?.unmount()); root = null;
  expect(revoked).toHaveBeenCalledExactlyOnceWith('blob:lys-installer');
});

it('keeps an encoded server file name and quotes it safely in the Arch command', async () => {
  const { host } = await draw(() => new Response('installer fixture', { headers: {
    'content-disposition': "attachment; filename=legacy.pkg.tar.zst; filename*=UTF-8''Studio%20O%27Brien.pkg.tar.zst",
  } }));
  await choose(host, 'arch'); await download(host);
  expect(downloads[0]?.name).toBe("Studio O'Brien.pkg.tar.zst");
  expect(host.querySelector('[data-installer="command"]')?.textContent).toBe("sudo pacman -U 'Studio O'\"'\"'Brien.pkg.tar.zst'");
});

it('keeps a leading dash in the server file name without turning it into a package-manager option', async () => {
  const { host } = await draw(() => file('-lys.pkg.tar.zst'));
  await choose(host, 'arch'); await download(host);
  expect(downloads[0]?.name).toBe('-lys.pkg.tar.zst');
  expect(host.querySelector('[data-installer="command"]')?.textContent).toBe('sudo pacman -U ./-lys.pkg.tar.zst');
});

it.each([
  [403, 'not_your_computer', 'this computer belongs to someone else', 'You do not manage this computer. Ask its owner or the administrator to make the change.'],
  [404, 'client_platform_unknown', 'no release for that kind', 'Lys has no installer for this kind of computer yet. Ask the administrator to add one.'],
] as const)('says the %s %s refusal in plain words and keeps its name in details', async (status, refusal, reason, words) => {
  const { host, updates } = await draw(() => new Response(JSON.stringify({ refusal, reason }), { status, headers: { 'content-type': 'application/json' } }));
  await choose(host, 'macos-pkg'); await download(host);
  expect(host.querySelector('[role="alert"]')?.textContent).toContain(words);
  expect(host.querySelector('[role="alert"] details')?.textContent).toContain(refusal + ': ' + reason);
  expect(downloads).toEqual([]);
  expect(created).not.toHaveBeenCalled();
  expect(updates).toEqual([initial]);
  expect(host.querySelector<HTMLButtonElement>('button[aria-label="Download the installer for Studio computer"]')?.disabled).toBe(false);
});

it.each([
  ['', 'installer fixture', 'InstallerFilenameUnreadable'],
  ['attachment; filename="../lys.pkg"', 'installer fixture', 'InstallerFilenameUnreadable'],
  ["attachment; filename*=UTF-8''bad%ZZ.pkg", 'installer fixture', 'InstallerFilenameUnreadable'],
  ['attachment; filename="lys.pkg"', '', 'InstallerDownloadEmpty'],
])('refuses an unusable file answer without inventing a file name', async (disposition, body, refusal) => {
  const { host } = await draw(() => new Response(body, { headers: { 'content-disposition': disposition } }));
  await choose(host, 'macos-pkg'); await download(host);
  expect(host.querySelector('[role="alert"] details')?.textContent).toContain(refusal);
  expect(downloads).toEqual([]);
  expect(created).not.toHaveBeenCalled();
});

it('names a transport failure and a refusal answer that cannot be read', async () => {
  let reachable = false;
  const { host } = await draw(() => {
    if (!reachable) throw new TypeError('connection refused');
    return new Response('no refusal body', { status: 502 });
  });
  await choose(host, 'deb'); await download(host);
  expect(host.querySelector('[role="alert"] details')?.textContent).toContain('ServiceUnreachable');
  reachable = true;
  await download(host);
  expect(host.querySelector('[role="alert"] details')?.textContent).toContain('Unanswered');
  expect(downloads).toEqual([]);
});

it('refuses a JSON answer in place of an installer and names a browser save refusal', async () => {
  let json = true;
  const { host } = await draw(() => json ? new Response('{}', { headers: { 'content-type': 'application/json' } }) : file('lys.pkg'));
  await choose(host, 'macos-pkg'); await download(host);
  expect(host.querySelector('[role="alert"] details')?.textContent).toContain('InstallerDownloadUnreadable');
  expect(created).not.toHaveBeenCalled();
  json = false;
  created.mockImplementationOnce(() => { throw new Error('downloads unavailable'); });
  await download(host);
  expect(host.querySelector('[role="alert"] details')?.textContent).toContain('InstallerSaveRefused');
  expect(downloads).toEqual([]);
});

it('names an interrupted file body through the shared request helper', async () => {
  const response = file('lys.deb');
  vi.spyOn(response, 'blob').mockRejectedValue(new TypeError('body interrupted'));
  const { host } = await draw(() => response);
  await choose(host, 'deb'); await download(host);
  expect(host.querySelector('[role="alert"] details')?.textContent).toContain('UnreadableResponse');
  expect(downloads).toEqual([]);
});

it('keeps JSON posts and named refusals unchanged when the same helper supports files', async () => {
  const exchanges = vi.fn().mockResolvedValueOnce(new Response('{"saved":true}'))
    .mockResolvedValueOnce(new Response('{"refusal":"not_your_computer","reason":"not its owner"}', { status: 403 }));
  vi.stubGlobal('fetch', exchanges);
  expect(await request('/network/machines', { name: 'Studio' })).toEqual({ saved: true });
  expect(exchanges.mock.calls[0]).toEqual(['/api/network/machines', expect.objectContaining({
    method: 'POST', body: '{"name":"Studio"}', credentials: 'same-origin', headers: { accept: 'application/json', 'content-type': 'application/json' },
  })]);
  await expect(request('/network/machines/' + machine + '/installer/deb', {}, 'POST', undefined, 'blob'))
    .rejects.toMatchObject({ status: 403, refusal: { refusal: 'not_your_computer', reason: 'not its owner' } });
  expect(exchanges).toHaveBeenCalledTimes(2);
});

it('keeps one download in flight and ignores its answer after the row closes', async () => {
  let answer: ((response: Response) => void) | undefined;
  const waiting = new Promise<Response>((resolve) => { answer = resolve; });
  const { host, exchanges } = await draw(() => waiting);
  await choose(host, 'deb'); await download(host); await download(host);
  expect(exchanges).toHaveBeenCalledTimes(1);
  expect(host.querySelector<HTMLSelectElement>('select')?.disabled).toBe(true);
  act(() => root?.unmount()); root = null;
  if (!answer) throw new Error('missing installer response signal');
  await act(async () => answer?.(file('lys.deb')));
  expect(downloads).toEqual([]);
  expect(created).not.toHaveBeenCalled();
});
