export function secretName(key: string, names: Map<string, string>): string {
  const generated =
    /^lys-app-((?:person|agent|op)-[0-9a-f]{32})-(.+?)-(api|client|custody)(?:-(?:op-)?[0-9a-f]{16,})?$/.exec(key);
  if (!generated) return key;
  const app = generated[2].replace(/[._-]/g, " ");
  const purpose =
    generated[3] === "api" ? "API credential" : generated[3] === "client" ? "Sign-in credential" : "Credential custody";
  return (
    app.charAt(0).toUpperCase() +
    app.slice(1) +
    " · " +
    (names.get(generated[1]) ?? "owner outside your view") +
    " · " +
    purpose
  );
}
