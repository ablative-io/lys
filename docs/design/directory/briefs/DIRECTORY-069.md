---
type: brief
id: DIRECTORY-069
cluster: directory
title: The administrator sets the message service on the Connections screen, with no terminal and no configuration file
---

# DIRECTORY-069: The administrator sets the message service on the Connections screen, with no terminal and no configuration file

> **Cluster:** directory
> **Depends on:** DIRECTORY-051
> **Design anchor:**
> - ADR-115 — Lys is the only sign-in a person or a product ever sees; the issuer inside it is never shown — Lys is the single sign-on for every product: every product is a client of Lys at Lys's own origin, and no product configuration names the issuer. A person meets only Lys screens: first-run setup, sign-in, provider setup and their own account are Lys pages, and the issuer's pages, admin site, name and password files are never part of any path a person follows. First run asks the person for the administrator's name, email and password; nothing is filled from the machine.
> **Checklist:**
> - C456 — The setting is Lys's own record, set on the Connections screen (DIRECTORY-069 R1).
> - C457 — An install's configured setting becomes the first line, once (DIRECTORY-069 R2).
> - C458 — The screen shows whether the service answers (DIRECTORY-069 R3).
> **Stories:**
> - S183 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As a person setting up Lys with no terminal, I want to connect my message service and bind its participants to my people and agents on a screen, so that the canvas shows who has spoken.

## Purpose

The canvas's message edges ask a message service which sessions have spoken (message_edges.rs, DIRECTORY-051). The service's address, the name of its session cookie and the explicit bindings of its participants to Lys people and agents are read only from the server configuration file (config.rs message_service, message_edges.rs Settings {url, cookie, bindings}), and the install writes that member only from an earlier configuration or a terminal flag (server_config.rs message_service, lines 174 to 215). With none set the edges answer that the service is not configured (message_edges.rs line 300). A person setting Lys up has no terminal, so today they cannot connect a message service at all, and a binding cannot be changed without editing a file and restarting.

## Task

Keep the message service setting as a record of Lys's own, set and changed by the administrator on the Connections screen and read by the message edges on each request. Carry an install's configured setting into that record once at upgrade, tested from an install made by an earlier build.

## Requirements

### R1: The setting is Lys's own record, set on the Connections screen

Behavioural. WHEN the administrator saves the message service on the Connections screen, THE SYSTEM SHALL keep its address, its session cookie name and its bindings as a line of a record Lys keeps in its own log store, written the way the apps' record is kept (apps_store.rs), and SHALL read the latest line on every message-edge request, so a change takes effect on the next request with no restart. The address must be absolute HTTPS, or HTTP on a loopback host. The cookie name must be a valid cookie token. Each binding names a participant id and a Lys person or agent the directory holds, and a participant or an identity bound twice is refused. Each refusal is named and shown on the screen beside the field it concerns. Only the administrator may read the bindings or save. Clearing the setting is a line of its own, after which the edges answer not set up, naming the Connections screen as the act. No secret is kept: the cookie is forwarded from the person's own browser, never stored.

**Acceptance:**
- The administrator saves an address, a cookie name and one binding on the Connections screen.
- The next canvas request uses the saved setting without a restart.
- An HTTP address that is not loopback is refused beside the address field.
- A binding naming an identity the directory does not hold is refused.
- A participant bound twice is refused.
- A person who is not the administrator is refused reading or saving the setting.
- Clearing the setting makes the edges answer not set up, naming the Connections screen.
- The record holds no cookie value.

**Files:**
- create: crates/lys-identity-server/src/message_settings.rs
- create: crates/lys-identity-server/src/message_settings_store.rs
- create: crates/lys-identity-server/src/message_settings_tests.rs
- create: surface/identity/src/features/connections/MessageService.tsx
- create: surface/identity/src/features/connections/MessageService.test.tsx
- modify: crates/lys-identity-server/src/message_edges.rs
- modify: crates/lys-identity-server/src/routes.rs
- modify: crates/lys-identity-server/src/config.rs
- modify: surface/identity/src/api.ts
- modify: surface/identity/src/features/connections/Connections.tsx

**Checklist:**
- C456 — The setting is Lys's own record, set on the Connections screen (DIRECTORY-069 R1).

**Stories:**
- S183 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As a person setting up Lys with no terminal, I want to connect my message service and bind its participants to my people and agents on a screen, so that the canvas shows who has spoken.

### R2: An install's configured setting becomes the first line, once

Behavioural. WHEN the server starts with a message_service member in its configuration and the record holds no line, THE SYSTEM SHALL record that setting as the record's first line, made by the start, and from then on SHALL read only the record. When the record already holds a line the configuration member is not read. The install and upgrade migration (server_config.rs) stops writing the member once an upgraded server has recorded it, and an earlier <service>_messages member is still carried as today. This is tested from a real install made by an earlier build whose configuration carries the bridge: after upgrade the canvas answers the same edges, the screen shows the carried setting, and a save on the screen replaces it.

**Acceptance:**
- An install made by an earlier build with a configured bridge shows the same setting on the Connections screen after upgrade.
- The same install answers the same message edges after upgrade.
- A second start records no second line from the configuration.
- A setting saved on the screen is not replaced by the configuration at the next start.
- An install with no configured bridge starts with no line.

**Files:**
- create: crates/lys-identity-server/src/message_settings_carry_tests.rs
- modify: crates/lys-identity-server/src/message_settings_store.rs
- modify: crates/lys/src/identity/install/server_config.rs

**Checklist:**
- C457 — An install's configured setting becomes the first line, once (DIRECTORY-069 R2).

**Stories:**
- S183 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As a person setting up Lys with no terminal, I want to connect my message service and bind its participants to my people and agents on a screen, so that the canvas shows who has spoken.

### R3: The screen shows whether the service answers

Behavioural. WHEN the Connections screen shows the message service, THE SYSTEM SHALL show the saved address, the cookie name, each binding by the person's or agent's name, and the outcome of asking the service's own status once for that page view, named in Lys's words: answering, refused with the service's status, or unreachable with the reason. Asking is a single request made when the page is opened or the setting saved, never repeated on a clock.

**Acceptance:**
- The screen names each binding by the identity's name.
- A service that answers is shown as answering.
- An unreachable service is shown as unreachable with the reason.
- Opening the screen asks the service once.

**Files:**
- modify: crates/lys-identity-server/src/message_settings.rs
- modify: surface/identity/src/features/connections/MessageService.tsx
- modify: surface/identity/src/features/connections/MessageService.test.tsx

**Checklist:**
- C458 — The screen shows whether the service answers (DIRECTORY-069 R3).

**Stories:**
- S183 (Person setting up Lys for the first time, Installs Lys on their own machine with no terminal knowledge and signs in) — As a person setting up Lys with no terminal, I want to connect my message service and bind its participants to my people and agents on a screen, so that the canvas shows who has spoken.

## Boundaries

- SHALL NOT need a terminal, a flag or a hand-edited file to connect, change or clear the message service.
- SHALL NOT store a cookie value, a token or any secret.
- SHALL NOT guess a binding from a matching display name.
- SHALL NOT repeat any request on a clock.
- SHALL NOT add a timeout, deadline, watchdog, poll, unsafe, ignored test or lint suppression.

## Verification

- Handwritten main brief passes the design gate on its exit code.
- Implementation follows card_build_v3, src_pr and src_land with Jev, fmt, Clippy pedantic, tests, ast-grep and the full gate on Dean.
- The upgrade leg starts from a real install made by build 1b568cd9 with a configured bridge and reads the setting back on the screen after upgrade.
