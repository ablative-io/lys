# ORGANISATION-ALONE: an organisation onboards itself

Written by Flapjack on 20 October 2026 at Lys main 9f5dfcc2 and Cambium main 37c2ea4bb, from the tree as read.
Every route and file:line below was read in those checkouts today; a step that does not exist says MISSING.
Design page only. Read whole by Waffles on 20 October 2026 and amended at 11:xx with his rulings (section "Rulings");
the briefs cite this page, not the message.

## Tom's bar, in his words

20 October 2026, 10:5x, by voice to Waffles: an organisation must be able to onboard itself,
"seamless and doable alone, without me and without one of my devices".

Read as four tests the walk below is judged by:

1. **Alone.** Nobody from ablative is present, signed in, or asked. No act in the path needs our administrator.
2. **Without Tom's devices.** Nothing is typed on a Mac of ours; no file is placed by hand; no credential is handed around.
3. **Seamless.** Each person receives one thing that lands in their hands and takes them to their place.
4. **Their own place.** Each person lands in their organisation's Cambium, not ours.

## The path end to end, as the organisation's own administrator would walk it today

Call her Priya, the administrator of an organisation that has never heard of us until this morning.

### Step 1. Priya arrives at lys.ablative.com.au and presses Sign in with Google

- The sign-in page and its routes: `crates/lys-identity-server/src/sign_in.rs:593-595` (`/login`, `POST /sign-in`, upstream providers);
  `sign_in_upstream.rs:348-357` (`/sign-in/providers`, `/sign-in/providers/{id}`, the provider callback on Lys's own origin,
  `sign_in.rs:249`); the OIDC callback `routes_table.rs:14` → `sign_in_callback.rs`; the code exchange `oidc.rs:215-264`.
- Lys tells the issuer (Rauthy 0.36.2) that nobody gets an account by signing in: `sign_in_providers.rs:559-563`
  (`"auto_onboarding": false, "auto_link": true`). Rauthy answers "User not found".
- Lys turns that into the refusal `NotAdmittedYet`, "This account has not been added to Lys yet. Ask your administrator to add it."
  (`sign_in_refusals.rs:31-34`, `sign_in_callback.rs:74-77`).

Priya's administrator is us. **MISSING: a way for a stranger to begin.** There is no route by which a person who is not yet
admitted can make an account, ask to be admitted, or found anything. (Test 1 fails at the first click.)

### Step 2. Priya's organisation comes into being

**MISSING.** Lys has no organisation a tenant could hang off:

- The only "organisation" in the identity server is the one settings zone of the install, read by `configuration_api.rs:87-88`
  and changed only by the administrator, `configuration_api.rs:100`.
- Teams are "a named group and nothing more: it carries no grant, and being in one confers no authority" (`teams_api.rs:7-9`;
  routes `teams_api.rs:259-265`).
- Budgets know `HolderKind::Organisation`, "every agent of the organisation", `budgets_state.rs:41-43`: the install, again.
- lys-secrets has a scope `Organisation(String)` (`crates/lys-secrets/src/store/scope.rs:14-22`, path `organisation/{name}`,
  `:33`): a label on a secret's path, bound to no record.
- The directory model has no organisation record, and a person record names no organisation.

### Step 3. Priya admits her own people

- The act is `POST /people/admit` (`routes_table.rs:29`; handler `people_admit.rs:205-237`), five steps in order
  (`people_admit.rs:35`): register the person in the directory, make the issuer account (`accounts.rs:148-170`, Rauthy
  `POST /users` with the email and names, no password), bind the login, activate, and optionally issue one root grant with
  `pass_on: use_only` (`people_admit.rs:327-343`).
- Its authority: `people_admit.rs:194-195`, `signed_in` then `crate::routes::administrator`. `routes.rs:336-344` admits the one
  configured administrator (`admission.rs:13-14`: one administrator login per install, never replaced) or an agent carrying a
  route grant (`routes.rs:350-370`). The route declares the scope `person.create` (`openapi_table.rs:166`) and the action exists
  (`crates/lys-identity/src/grants/shipped.rs:56`), but a person holding a `person.create` grant does not pass this route: the
  check never consults grants.

**MISSING: admission by anyone but our administrator.** (Test 1 fails again.)

### Step 4. Each person receives something that lands in their hands

**MISSING.** Admit sends nothing:

- The "invitation" of DIRECTORY-101 is text the administrator copies by hand from `surface/identity/src/features/people/AddPerson.tsx:106-109`
  ("Hello <name>, you have been added to Lys. Open <address> and choose Sign in with Google, using <email>."), with a
  `Copy invite` button and "Copied. Send it to <name>." (`AddPerson.tsx:135-136`), and the line "They have no access yet. Give it
  from Access once there is something to give." (`:138`).
- Lys has no mail path: no mail crate in any `Cargo.toml`, every `notify` is `tokio::sync::Notify`, and `message_service.rs:1-6`
  is a read bridge to a message service's registry, not a sender.
- The issuer could send mail, and does not here. Rauthy's own documentation says an account made by an administrator gets a
  first-password mail only when SMTP is configured, and that registration mail needs `SMTP_URL`, `SMTP_USERNAME` and
  `SMTP_PASSWORD` set. Lys writes the issuer's environment itself: the compose template `deploy/identity/compose.yaml:84-117` and
  the generated env `crates/lys/src/identity/prepare.rs:216-245` set seventeen `RAUTHY_` keys (PUB_URL, RP_ID, RP_ORIGIN,
  PROXY_MODE, COOKIE_MODE, TRUSTED_PROXIES, PEER_IP_HEADER_NAME, the four BOOTSTRAP keys, the ENC and HQL keys, LISTEN_PORT,
  DB_PASSWORD) and no SMTP or mail key of any name. The installed compose file on this Mac was read for key names only
  (`~/Library/Application Support/lys/identity/deploy/compose.yaml`): the same seventeen, no SMTP. `deployment.toml` and the
  template `crates/lys/src/identity/install/deployment.template.toml` carry sections `[deployment] [issuer] [database] [spicedb]
  [credentials] [clients.platform]` and no mail section. The only email key anywhere is `admin_email`
  (`crates/lys/src/identity/config.rs:70-74`), for the unattended install's administrator.

So today a person admitted by Google gets nothing from Lys and nothing from the issuer; a person admitted with a password
gets a password the administrator set and sent by hand. (Test 3 fails.)

### Step 5. Priya gives her people Cambium

- The act is `POST /grants/roots` (`grants.rs:286`; handler `grants/handlers.rs:105-122`). Its authority is stricter than
  "administrator": the engine refuses any caller but the one root authority person,
  `crates/lys-identity/src/grants/admission.rs:277-281` (`RootAuthorityRefused`), reached through `grants/mode.rs:45-69` and
  `authority.rs:314-332`; the root authority is the administrator's person (`agent_roots.rs:151-158`, passed at `grants.rs:134,172`).
- Grants do carry pass-on: `PassOn::UseOnly` or `PassOn::To { actions, recipients }` with recipients among Person, Agent,
  ServiceAccount, Connector, Machine (`crates/lys-identity/src/grants/types.rs:204-255`); the administrator's own setup roots
  pass on to people and agents (`agent_roots.rs:135`). Delegation (`POST /grants`, `grants.rs:284`, judged by
  `delegation_authority`, `grants/handlers.rs:124-179`) and revocation (`POST /grants/{id}/revoke`, `grants.rs:323`,
  `revocation.rs:71`) exist. But a delegation passes on an existing grant to an identity that already exists; it cannot admit
  anyone, and it cannot issue a root.

**MISSING: a root authority that is Priya for her organisation's resources.** (Test 1.)

### Step 6. Each person signs in to Cambium and lands in their organisation's own place

- The door admits every sign-in against the one `state.settings.workspace_id`
  (`crates/cambium-door/src/http/oidc_admission.rs:73` minimum revision, `:80` projection asked of Lys for that workspace,
  `:101` the grant check `cambium.workspace read`, `:166` channels and topics kept only in that workspace, `:243` membership
  for refresh, `:275` the membership record). A person Lys does not admit to that one workspace is refused
  "you do not have permission to open this workspace" (`:29, :88, :147, :155`).
- The workspace id is configuration: `registry.workspace_id` (`crates/cambium-server/src/config/types.rs:284-285`), default
  `"cambium"` (`config/defaults.rs:158-159`), and the workspace comes into being at the first `serve` when the boot channel is
  minted (`crates/cambium-server/src/boot/passes.rs:37-41` → `crates/cambium-door/src/http/place.rs:160-178`). No route and no CLI command creates one (`cli/args.rs:43-82`:
  Mcp, Serve, Backup, SettlePromotions, Cutover). The store can hold several workspaces (`field_builtin.rs:308`) but the door
  serves one.

So Priya's people, once admitted to our workspace, would land in **our** Cambium. **MISSING: a workspace that is the
organisation's.** (Test 4.)

### Step 7. The link between Cambium and Lys

A correction to what was believed this morning: the two credential files are not hand-made.

- The door's connect flow (`http/lys_connect.rs:1-6`, routes `/identity/lys/connect` and `/identity/lys/connected`,
  `:325-326`) exchanges a one-time code with Lys and writes `client.credential`, `workspace.credential` and `lys-keys.json`
  owner-only, temp file then rename, and rewrites the config keys `identity.workspace_credential_path` and
  `[mailbox.lys].keys_path` (`http/lys_connect/credentials.rs:56-87`; the key set is fetched from `{lys}/api/oauth/jwks`,
  `http/lys_connect_keys.rs:1-33`). Documented defaults: `docs/CONFIG.md:309` (`<data>/identity/workspace-app.secret`) and
  `:288` (`keys_path`, no default).
- Lys's side is `POST /apps/connect/exchange` (`apps_connect.rs:67`, answer `:111-116`), and it is administrator-only
  (`apps_connect.rs:268, :324`).

What is MISSING here is narrower than "nothing writes them": the approval is our administrator's (test 1), and the
credential is one per door install, not one per organisation. The door is our service, connected once; what must become
the organisation's is the workspace it serves them, not the door's own credential.

### The walk in one line

Of seven steps, one exists whole (7, with our approval), two exist for our administrator alone (3, 5), and four are
MISSING outright (1, 2, 4, 6). Nobody has pressed this path through for one person because it cannot be pressed: it stops
at the first click.

## The organisation as a resource in Lys

**What it is.** A directory record, `organisation`, with an id, a name, a founding person and a state (Active, Retired),
in the same log as people and agents; and a kind, `organisation`, in the grants model (`shipped.rs`) with its actions:
`organisation.read`, `organisation.admit` (admit a person into it), `organisation.grant` (issue a root on a resource it
owns), `organisation.administer` (change its name, retire it, pass administration on). A person record carries the
organisation it was admitted into. Our own people are an organisation too: the install's, founded by the unattended
install from `admin_email` (`crates/lys/src/identity/config.rs:70-74`) with our administrator as its founding person. One path,
not two: a path we never walk is the path that breaks for Priya (Waffles, ruling 1). A resource issued under an organisation
(a Cambium workspace, a team, a secret scope) carries the organisation's id; `organisation/{name}` in lys-secrets becomes
that id rather than a free label.

**Who administers it.** The founding person holds the `administrator` role on `organisation:<id>`, issued at the moment of
founding by the system itself (the one act where Lys, not a person, is the issuer; recorded as such), with
`PassOn::To { actions: [organisation.admit, organisation.grant, organisation.administer], recipients: [Person] }`.

**What pass_on means for it.** Exactly what `PassOn::To` means today (`types.rs:245-255`), with one rule added at judging:
a grant passed on under an organisation may name only recipients admitted into that organisation, and only resources that
carry that organisation's id. Priya may make a second administrator of her own people; she may not pass anything to one of
ours, and nothing of hers reaches a resource that is not hers. A revoke by her of a grant she passed on is today's
`revocation.rs:71` rule unchanged (the issuer may revoke).

**How the administrator-only routes become organisation-scoped without widening what our administrator can do.**

- `POST /people/admit` takes an `organisation` field in its body. Its check becomes: without the field, today's
  `routes::administrator` (`routes.rs:336`), unchanged; with it, the caller must hold `organisation.admit` on that
  organisation (a grant check, the same engine the door asks), and the admitted person is written with that organisation's
  id. Our administrator holds no grant on any organisation and so cannot admit into one: the route widens for Priya and
  narrows nothing and widens nothing for us.
- `POST /grants/roots`: `judge_root` (`admission.rs:273-281`) today compares the caller with one `root_authority: PersonId`
  (`authority.rs:121`). It becomes a function of the resource: for a resource carrying an organisation id, the root authority
  is the holder of `organisation.grant` on that organisation; for an install-level resource, today's person. Our
  administrator stays root authority for our resources and is not one for Priya's.
- Every read route that lists people, agents, teams and grants filters by the caller's organisation, with no special case
  for ours: ours see ours, Priya sees hers. The install's directory projection stays one log; the view is partitioned.
  `routes::administrator` stays for install-level acts only: apps, the settings zone, the issuer.
- Teams: unchanged in authority (`teams_api.rs:7-9`), gain an organisation id.

**Founding.** The first act of all, by a person who has no account: `POST /organisations/found` (unauthenticated) with an
email and the organisation's name. Lys makes the issuer account (`accounts::make`, which already answers an existing account
by email, `accounts.rs:153`) and the person record in state `Founding`, writes the organisation record in state `Founding`,
and sends the person one mail (see the mail seam) with a link that proves the address; following it activates the person,
the organisation, and the founding grant, and signs them in (Google through `auto_link`, or a password they set on that
page under Lys's own policy, `accounts.rs:18-24`). Until the address is proven, nothing is active and the names are not
taken: a second founding with the same email and name re-sends the mail (idempotent on the operation id, as admit is).

## The Cambium side

**One workspace per organisation, in the one door.** Not one Cambium install per organisation: that needs our hands on a
device for each (test 2 fails) and the door's store already holds several workspaces (`field_builtin.rs:308`). The door
stops serving one configured id and serves the workspace Lys names for the person.

**Made by whom and when.** The workspace exists in Lys first: founding an organisation issues, under it, a resource
`workspace:<organisation id>` with the founding person as root authority through `organisation.grant`. The door mints the
workspace's boot channel the first time an admitted person of that organisation enters (`place::boot_channel(store,
workspace_id)`, `place.rs:160-178`, called per workspace rather than once at serve), and from then on that person's live
prefix, channels, topics, drafts and search are scoped by that id as `oidc_admission.rs:166` scopes channels and topics
today, extended to every read. `registry.workspace_id` becomes the install's own organisation's workspace, the one our people enter; no person is
without an organisation.

**The admission question the door asks.** `oidc_admission.rs:80` asks Lys the projection for one workspace id. It becomes
"which resources of kind `cambium.workspace` may this subject read", and the route exists today: `POST /grants/which`
(`crates/lys-identity-server/src/grants.rs:296`, `grants_batch.rs:1-14`: "answers, a page at a time after a cursor, the
ids of a kind a subject may take an action on", open to an app through its credential for its own kinds). The door asks it
once at sign-in and opens the person's workspace. The sign-in link carries nothing: a link that carries anything
authority-shaped gets forwarded and goes wrong (Waffles, ruling 6). A person has one organisation and so one workspace.

**The two credential files.** They stay the door's, written by its own connect flow (`lys_connect/credentials.rs:56-87`)
after our administrator approves the door once (`apps_connect.rs:268`). That is the one act of ours in the whole path, done
once per door install and never per organisation, so it does not break test 1 for Priya. The app credential authorises the
door to ask Lys about any subject and any workspace; per-organisation authority lives in Lys's grants, not in a file.

## Briefs this page yields, one per seam, Lys first (Tom's rule: Lys identity comes first)

Ids are the next free from origin/main as read today (DIRECTORY-108, ACCESS-008, IDENTITY-003 drafted, CORE-072); whoever
writes each takes the next free id at that moment and keeps the order.

1. **DIRECTORY-109 (lys): the organisation record and kind.** The `organisation` directory record and its states; a person
   record's organisation id; the kind and its four actions in the grants model; the founding grant issued by the system;
   the read routes partitioned by organisation. Files: `crates/lys-identity/src/directory/` (records, projection),
   `crates/lys-identity/src/grants/shipped.rs`, `crates/lys-identity/src/grants/types.rs`,
   `crates/lys-identity-server/src/read_api.rs`, `teams_api.rs`, `openapi_table.rs`, `routes_table.rs`, the surface's
   people and teams features.
2. **ACCESS-009 (lys): organisation-scoped admission and roots.** `POST /people/admit` with an organisation; the grant check
   beside `routes::administrator`; `judge_root`'s root authority per resource; the pass-on rule bounded to the organisation.
   Files: `crates/lys-identity-server/src/people_admit.rs` (:194-195, :327-343), `routes.rs` (:336-344),
   `crates/lys-identity/src/grants/admission.rs` (:273-281), `authority.rs` (:121, :314-332), `mode.rs` (:45-69),
   `grants/handlers.rs` (:105-179), `agent_roots.rs` (:135, :151-158), `revocation.rs` (:71); and Priya's screens, or she
   has routes and no door: People and Access scoped to her organisation in `surface/identity/src/features/people/` and
   `surface/identity/src/features/access/` (the add-person act names her organisation; the access screen gives only what
   she holds under it; our administrator's screens show only ours).
3. **DIRECTORY-110 (lys): mail** (written as the page's IDENTITY-004; the identity directory sits outside the gate). The install carries a `[mail]` section (SMTP host, port, user, from, TLS mode) in
   `deployment.toml`; `prepare.rs` writes it to the issuer as `RAUTHY_SMTP_*`; the compose template carries the keys; Lys
   itself sends its own mail through the same settings (one sender crate, one template per letter: founded, added, access
   given), and a missing `[mail]` section makes founding and admit refuse by name rather than copy text to a clipboard.
   `AddPerson.tsx`'s copy-invite goes. Files: `crates/lys/src/identity/install/deployment.template.toml`,
   `crates/lys/src/identity/config.rs`, `prepare.rs` (:216-245), `deploy/identity/compose.yaml` (:84-117),
   `crates/lys-identity-server/src/people_admit.rs`, a new `mail.rs`, `surface/identity/src/features/people/AddPerson.tsx`
   (:106-138).
4. **DIRECTORY-111 or the next free (lys): founding.** `POST /organisations/found`, the proving link, the founding page on the public sign-in
   surface, the Founding states and their activation, idempotence on the operation id. Depends on 1, 2, 3. Files:
   `crates/lys-identity-server/src/sign_in.rs` (:593-595), `sign_in_callback.rs` (:58-77), `sign_in_refusals.rs` (:31-34),
   `session_admission.rs` (:31, :56-57), `accounts.rs` (:148-170), a new `organisations_found.rs`, the surface sign-in page.
5. **CORE-073 (cambium): a workspace per organisation in the one door.** The admission question becomes "which workspaces";
   the boot channel per workspace at first entry; every read scoped by workspace, proven red first, one test per surface, that two organisations'
   people never see one another's channels, topics, drafts, search, presence, huddles, files or MCP seats, named one by
   one. The admission question is `POST /grants/which` for kind `cambium.workspace`, action read (no Lys change needed). Files: `crates/cambium-door/src/http/oidc_admission.rs`
   (:29-101, :166-178, :243-285), `http/place.rs` (:160-178), `crates/cambium-server/src/boot/passes.rs` (:37-41), `crates/cambium-server/src/config/`
   (`types.rs:284-285`, `defaults.rs:158-159`), `boot/settings.rs` (:61, :165), `http/field_builtin.rs` (:308), the live
   prefix and search paths that read `workspace_id`.

Order: 1, 2 and 3 can be written at once and land in any order; 4 needs all three; 5 needs 1 and 2 on a published Lys and
can be written now against their contract. No brief is written before Waffles reads this page.

## Rulings (Waffles, 20 October 2026, by Tom's rules; the reasons are his)

1. Our own people are an organisation too: the install's, founded by the unattended install from `admin_email`, our
   administrator its founding person. One path, not two; `routes::administrator` for install-level acts only (apps, the
   settings zone, the issuer); every people, teams and grants read filters by organisation with no special case.
2. No standing power for our administrator inside an organisation; a grant Priya can revoke is the only way in.
   "Without me" is the bar.
3. Mail: the install's own SMTP, no mail API, with two rules. (a) One letter per act reaches the person, never two; the
   issuer's own mail must be on, because forgot-password has to work with nobody from us present; if Rauthy's
   first-password mail on `POST /users` duplicates Lys's letter and cannot be turned off per account, DIRECTORY-110 decides
   by test which one goes and says so. (b) The gate proves letters against a capturing SMTP sink in the test, every letter's
   text asserted; no live mailbox in a test, ever.
4. One door, many workspaces.
5. An organisation of one is valid; the proven address is the confirmation.
6. The sign-in link carries no organisation. The door asks Lys (`POST /grants/which`) and opens the person's workspace.
7. Organisation names are labels; the id is the key. No uniqueness is claimed or enforced for a name beyond the founding
   person's own: two organisations may share a name and are told apart by id everywhere.
8. One person, one organisation. The person record carries one organisation id; if two are ever needed, that id becomes a
   list in DIRECTORY-109 and nothing else on this page moves.

## Questions the page could not settle from Tom's rules, each with the recommendation (all four taken above)

1. **May our administrator act inside an organisation at all (support)?** Recommendation: no standing power. Priya may
   pass `organisation.administer` to one of ours by a grant she can revoke; that is the only way in, and it is on her record.
   Tom's bar is "without me": a support back door is the thing the bar refuses.
2. **Who sends the mail?** Recommendation: the install's own SMTP settings, written once to the issuer and used by Lys's
   own sender; no third-party mail API. The proving link and the invitation are both Lys's letters, so the text is ours and
   the refusal when mail is unconfigured is named, never a silent clipboard.
3. **One Cambium per organisation, or one door with many workspaces?** Recommendation: one door, many workspaces. The
   store already holds several; a per-organisation install needs our hands for each organisation and fails test 2.
4. **Does founding need a second person's confirmation (an organisation of one)?** Recommendation: no. An organisation of
   one is a valid organisation; the proven address is the confirmation.

## What this page does not prove

- Rauthy 0.36.2's behaviour on `POST /users` without SMTP was read from its documentation, not run; whether its
  first-password mail is the right letter for a Google-linked account is not decided here (DIRECTORY-110 decides, by test).
- That the door's reads are scoped by workspace everywhere: `oidc_admission.rs:166` scopes channels and topics; search,
  drafts, presence and the live prefix were not read for it today. CORE-073's red proves or disproves each.
- The cost of a grant check per sign-in per organisation at scale (the 14 October sign-in cost, rights_claim::pass over
  30 s, is on record); no measurement here.
- The public path for a stranger (lys.ablative.com.au, Google through Rauthy with `auto_link`) was not walked today; the
  refusal at step 1 is read from the code, not seen in a browser.
- One person, one organisation (ruling 8): the record carries one id; a list if ever needed, nothing else moves.
- The organisation's agents. Priya's people will enrol and start agents under AGENTS-017, and those agents need seats in
  her workspace, not ours: the agent's responsible person carries the organisation, so the agent does; its roots
  (`POST /grants/agent-roots`, `grants.rs:287`, `agent_roots.rs`), its machine and provisioning records, and the door's seat
  for it (`cambium-door` agent seats under `http/agent_seat/`) must all be scoped by that organisation. Not a brief today;
  named here so it is not forgotten, and it is the sixth brief once DIRECTORY-109 and CORE-073 have landed.
