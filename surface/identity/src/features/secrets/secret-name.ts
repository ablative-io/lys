const GENERATED = /^lys-app-((?:person|agent|op)-[0-9a-f]{32})-(.+?)-(api|client|custody)(?:-(?:op-)?[0-9a-f]{16,})?$/;

/** The app a secret Lys keeps for an approved app belongs to, by the name Lys generated for it; null for any other secret. */
export function appOfSecret(key: string): string | null {
  return GENERATED.exec(key)?.[2] ?? null;
}

export function secretName(key: string, names: Map<string, string>): string {
  const generated = GENERATED.exec(key);
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
