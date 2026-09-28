# DIRECTORY-054 R5: Lys from the disk image to signed in, on a fresh macOS account

**Status: not run.** The run needs two things this row cannot make for itself, and both
wait for Tom:

- **Tom's Developer ID.** Only an app signed with it and notarised passes Gatekeeper on a
  machine that has never held Lys, and only such an app carries the release name `Lys`
  (`lys package app` refuses `signing_identity_missing` without it and writes no `Lys.dmg`).
- **The fresh account.** It is a macOS virtual machine on Tom's Mac, made for this proof. It
  is never a new account on Dean's laptop, which belongs to Tom's father. Making it waits for
  Tom's word.

This file holds the procedure the run follows and, once the run is made, its record. Nothing
below is a claim that the run has happened.

## What is proved

A person who has never used a terminal downloads `Lys.dmg`, drags Lys to Applications, opens
it, completes first-run setup and signs in, and:

1. the recorded run reaches signed in from the disk image;
2. the process record holds no terminal or shell started by the person or by Lys on their
   behalf; and
3. a text scan of every page shown finds no name of the issuer inside Lys.

## Before the run

1. On a machine with Tom's Developer ID in its keychain and the notarisation credentials
   stored under a keychain profile, build every Lys binary for both processors from one
   commit, build and package the screens from that commit, and run:

   ```
   lys package app --out dist --arm64 target/aarch64-apple-darwin/release \
     --x86-64 target/x86_64-apple-darwin/release --surface surface/identity/package \
     --developer-id "Developer ID Application: …" --notary-profile lys-notary
   ```

   Record the build line it prints and the output of
   `codesign --verify --deep --strict dist/Lys.app`, `spctl --assess --type execute dist/Lys.app`
   and `lipo -archs` on each binary in `dist/Lys.app/Contents/MacOS`.
2. Make the macOS virtual machine on Tom's Mac with a new user account that has never held
   Lys, with Docker Desktop not installed.
3. Put `Lys.dmg` where the person downloads it from (a link in a browser, as a person would
   receive it).

## The run

1. Start a screen recording in the virtual machine.
2. Start the process record: every process started in the account for the length of the run,
   with its parent, as the system's own endpoint-security or audit log records it, kept
   outside the account the person uses.
3. The person downloads `Lys.dmg`, opens it, drags Lys to Applications, and opens Lys from
   Applications.
4. The progress page shows each step. When it shows the container engine's guidance, the
   person follows it: downloads Docker Desktop from the link on the page, installs it and opens
   it. The install continues by itself.
5. The browser is handed to first-run setup; the person completes it and signs in.
6. Stop the recording and the process record.

## The checks on the record

- The recording reaches the signed-in screen.
- The process record, filtered to processes whose ancestry reaches the person's session or
  Lys, lists no `Terminal`, `iTerm`, `sh`, `bash`, `zsh`, `osascript` or other shell or
  terminal.
- Every page the browser showed during the run is saved as text and scanned
  case-insensitively for the issuer's name; the scan finds nothing.

## Record

Not yet made. When the run is made, this section holds the date, the build line, the
recording's and the process record's locations and digests, the filtered process list, and
the scan's output, each as the tool printed it.
