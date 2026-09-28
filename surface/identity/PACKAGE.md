# Package the identity screens

From clean committed surface source with dependencies installed, run:

```
npm run package -- /absolute/path/to/new-surface-package
lys identity install --surface /absolute/path/to/new-surface-package
```

Packaging checks TypeScript and makes a fresh production build. It refuses dirty source, a changed source commit, symlinks, missing compiled assets, and an existing destination. It writes `surface-manifest.json` last: format `lys-identity-surface/v1`, source commit, and every file's relative path, byte length and SHA-256. The installer verifies that manifest. A partial copy without its manifest cannot be installed.

Node and npm are build tools only. The installed identity service serves these screens and `/api` on the same origin, without Node or Vite at runtime. Packaging does not change the live installation.
