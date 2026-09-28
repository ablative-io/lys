# Lys-Log-Store — User Stories

## Consumer library — Settling an uncertain append by reading the leaf back

**S1.** As the identity directory committing through the file store, I want a leaf that reads back under its final name to be whole and flushed so that settling an uncertain append by reading the leaf back gives the committed answer.

## Operator — Reopening a log after a crash or a failed write

**S2.** As an operator reopening a log after a crash, I want only whole, flushed leaves to be counted so that a torn leaf is never pinned into the tree.

## Writer — Racing another writer for the same index

**S3.** As a writer whose index another writer has already taken, I want my write refused by name so that neither writer's leaf is replaced.

## Writer — Seeing a failure after its leaf was named

**S4.** As a writer whose append failed after its leaf was named, I want an error that says the leaf is written but its durability is uncertain so that I never mistake my own leaf for another writer's.

## Read-only caller — Opening a store without writing to it

**S5.** As a read-only caller opening a store, I want open to leave every file in the directory as it found it so that a read-only open stays read-only.

## Read-only caller — Reading a log on read-only media

**S6.** As a reader of a log on media that refuses a directory flush, I want a read-only open to count the named leaves without flushing so that a status opens as it did before this change.

## Read-only caller — Reading a log that a crash left one leaf ahead of its pin

**S7.** As a reader of a log one leaf ahead of its pin, I want a read-only open to leave the store as it found it and tell me that a writable open repairs it so that reading never writes and the pending repair is never silent.
