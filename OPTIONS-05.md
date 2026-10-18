# Domain 05 options: sign-in, You, Settings, setup

Each item from the Top ten of `05-person-entry-settings.md`, in order. For each: the options, what Cherie (rung 1, first MacBook this weekend) would see, the review mistake each avoids, which purpose it serves (what purpose, what problem, what value), the recommendation, and a second go after looking at it in the dev server (http://127.0.0.1:5185/).

## Item 1. Signing in takes two screens

Today a signed-out page shows a small card with one "Sign in" button; it goes to `/api/login`, which only redirects to `/#/sign-in`, a second screen with a different layout and the real form.

| Option | What Cherie sees | Mistake it avoids | Purpose served |
|---|---|---|---|
| A. Every signed-out page shows the one sign-in form in place, at the address she opened; once signed in, that same page opens. The card is deleted. | One screen: Lys, "Sign in", her fields, "Sign in with Google". The invitation link she opened is the page she ends on. | Two `SignIn` components (review duplication table); a lost place after sign-in. | Problem: she gets in on the first try. Value: the link she was sent works as a link. |
| B. Keep the card, but its button goes straight to `#/sign-in` without the server hop. | Still two screens, one press shorter. | Only the server round trip. | None new: the two layouts stay. |
| C. Every signed-out page redirects to `#/sign-in` and remembers where it came from. | One screen, but the address in her bar changes, and the way back depends on browser storage. | Two components. | Problem solved, but adds a third "where you land" path, the duplication item 3 removes. |

Recommendation: **A**. It removes a component rather than adding a path, and keeps the address she was given. The Google button now remembers a file link before leaving (the old card did this for its button).

Second go. Looked at `#/me`, `#/people` and `#/sign-in` signed out against the installed Lys: all three are now the same screen. The first render inside the shell stretched the form to the page's height (the shared `.page:not(.fill) > form` rule), fixed in `sign-in.css`. What was still wrong, seen as Cherie: the line under the heading said "with your email and password" to someone invited to use Google; the help line named the administrator twice and said "admit"; and if the providers list could not be read, Google simply was not there, with no word. I did **not** move Google above the password fields: the providers answer arrives after the form, so putting it on top would push the fields down under her cursor as she types. Instead the line under the heading now names every way in ("With Google, or with your email and password."), a failed providers read says so and that the password still works, and the help line is one sentence ending in the administrator's email as a mailto link. Still wrong, and left for item 3/9: Google is styled as the secondary button although it is the invited path, and the administrator's name arrives late and shifts the help line.

## Item 2. No sign-out except an unlabelled square on You

Today the only way out is a 32 px filled square (the agent-stop symbol) in the sessions table on You, with its word hidden by `you.css`. The rail and the palette have none.

| Option | What Cherie sees | Mistake it avoids | Purpose served |
|---|---|---|---|
| A. One `SignOut` control (in `features/me`): the word "Sign out", one press, no confirm; it ends this browser's session and opens the sign-in page saying "You have signed out". Placed at the top of You now, and asked of the shell for the foot of the rail with her name. The sessions table shows its words. | "Sign out" in words beside her name on You, and (once the shell takes the ask) at the foot of the menu on every page. | A symbol with no word for an act a person must find; one way out hidden in one table. | Value: she can leave a shared or borrowed computer safely, which is what a sign-in promises. |
| B. Only ask the shell for a rail item; change nothing on You. | Nothing until domain 01 lands it. | Same, later. | Same value, but nothing she can see today. |
| C. Keep the sessions-table button, just show its word. | "Sign out" in the middle of a table of sessions she has to scroll to. | The hidden word only. | Half: the act exists but is not where anyone looks. |

Recommendation: **A**. The act lives in my folder so the shell's change is one line, and You shows it today. No confirmation: signing out loses nothing she typed and is undone by signing in, so a confirm is friction on the right act (CONSENSUS: make the right thing easy first).

Second go. Looked at You on a stand-in service built from the test fixtures (port 5187; I may not sign in to the installed Lys). "Sign out" now sits top right of You beside the name. With the word showing, the sessions table's buttons spilled past the card's edge, which is why the old CSS had hidden the word; I widened the action column instead of hiding the word again. What is still wrong: (1) the header act uses the back chevron, which reads as "go back"; there is no leave symbol, so I asked the shell for one rather than mirror `enter` (mirrored, it still points into the door). (2) The sessions table still draws the current row's "Sign out" with the agent-stop square; that file is not mine, so it is an ask. (3) The real fix is the rail foot, on every page; that is the shell's, and the ask is one line because the act lives in `features/me/SignOut.tsx`. The sign-in page it opens now says "You have signed out of Lys in this browser." so the person knows it worked.

## Item 3. Landing differs by route

Today Google sign-in lands on `/#/me` (fixed in the server, `sign_in_upstream.rs:49`), password and first-run setup on `/#/`, and the issuer callback on `/#/`. The invited person, who uses Google, lands on You, where nothing says what to do next.

| Option | What Cherie sees | Mistake it avoids | Purpose served |
|---|---|---|---|
| A. Every sign-in lands on Home, the page CONSENSUS makes answer "what is waiting on me" first; a sign-in begun on a page (item 1's in-place form) lands back on that page, whichever way she signs in. Google's fixed return to `/me` is turned into the same landing in the browser: the Google button remembers where she was going, and the return to `/me` goes there. | Press "Sign in with Google" on the sign-in page, choose her account: she is on Home. Open an invitation link, sign in with Google: she is on the page the link named. | Three landing addresses (review duplication table); the invited path being the one that never sees what to do first. | Purpose: the first screen after sign-in is the one built to say what to do. Value: links she is sent survive sign-in either way. |
| B. Make You the landing for every sign-in and add "what Lys is for" to You. | Her name and a paragraph on You. | Three addresses. | Contradicts CONSENSUS (the first thing seen answers "what is waiting on me", which is Home), and You is for her account, not for next steps. |
| C. Ask the server to land Google on `/#/` and change nothing in the browser. | Nothing today; then Home, but a Google sign-in begun from a link still loses the link. | Three addresses, later. | Half: no "back to where I was". |

Recommendation: **A**, with no server change: the server's `/me` return becomes a hop the browser takes once, and only when a Google press in this tab asked for it, so a later visit to You is never redirected. If the server's landing is ever changed, `fileReturn.tsx`'s `/me` changes in the same commit (written as a note in CONSENSUS, not an ask).

Second go. The first version took the hop in a layout effect, to keep You from flashing. In the browser the address moved to `#/` while You stayed on screen: on first mount the router is not yet listening, so it never heard the change. The jsdom tests passed anyway, because they asserted only the address, and they still pass with the broken version, because jsdom's ordering differs. That is a test agreeing with itself. The hop is now an ordinary effect, and the tests assert what is drawn (Dashboard, People and agents, or You), not only the address. The browser is the check that tells the two versions apart, and it now shows Dashboard, or People when the Google press began there. A visit to You afterwards stays on You. What is still wrong: Home is the right landing only once it says what to do first, and that is domain 06's page. You still shows for one frame on a provider return before the hop. That flash ends only if the server lands providers on `/#/`, which I did not ask for, because the browser fix already gives one landing and the server change has to move with `fileReturn.tsx`.

## Item 4. Google sign-in works only at one address, and says the wrong thing elsewhere

Begin Google at any address but the configured one (a LAN IP, a tailnet name, localhost) and the provider returns to the configured address without the browser-binding cookie. The person lands on the sign-in page there with "That sign-in expired or was already used. Try again." That is false. Read in the server: the refusal's redirect is relative, so that page is already the right address, and trying again from it works. The first attempt from the wrong address never can.

| Option | What Cherie sees | Mistake it avoids | Purpose served |
|---|---|---|---|
| A. Now: the words name both real causes and say that this page is the place to try again, naming its address. When the server says its own address in `/sign-in/providers` (server ask), a page opened at another address says so before she presses Google and links to the right address, keeping the page she was on. The browser handles the answer with or without the new field. | Today: "That sign-in could not be finished. It began at a different address from this one (lys.example), or it expired or was already used. Sign in again on this page." After the server change: "Sign in with Google works only at lys.example. You opened Lys at 192.168.1.4:8490." with an "Open Lys at lys.example" button. | A false error that sends her round in a loop (CONSENSUS: every error names the real cause and the next act). | Problem: she gets in from whatever address she was given. |
| B. The words only. | The honest sentence, after one failed try. | The false error. | Half: still one failed attempt, every time. |
| C. Hide Google when the address is wrong. | No Google button at all, and no reason why. | The failed attempt. | Hides the invited way in from a rung-1 person: the worst outcome. |

Recommendation: **A**. The words land today. The early warning is ready in the browser and switches on when the server field exists, and nothing is hidden.

Second go. Looked at both against real pages. The installed Lys, with `?refused=SignInStateUnknown`, now shows "That sign-in could not be finished. It began at a different address from this one (127.0.0.1:5185), or it expired or was already used. Sign in again on this page." On the stand-in, with `origin` set to another address, the Google button is replaced by "Sign in with Google works only at lys.ablative.com.au. You opened Lys at 127.0.0.1:5187." and "Open Lys at lys.ablative.com.au". What is still wrong: (1) the early warning stays off until the server ask lands, so today Cherie still burns one attempt at a wrong address, though the next one now works. (2) "Open Lys at …" carries the page she was on, but if that page was a sign-in refusal its `refused=` goes along, and she would see the old error at the new address; that is rare and harmless, so I left it. (3) The words still say "This Google account" for any provider's not-admitted refusal (review Words table); the refusal does not name the provider, so I left that for a server field rather than guess.

## Item 5. Settings disappears for anyone who is not an administrator

Settings is gated on `/configuration`, which only administrators may read, so a non-administrator loses Settings from the rail, and with it Layout (menu side and labels, kept per browser) and Your account. Layout sits on the administrators' page.

| Option | What Cherie sees | Mistake it avoids | Purpose served |
|---|---|---|---|
| A. Split by whose the setting is. `#/settings/account` becomes "For you": her sign-in email and password, and Layout ("in this browser"), for everyone. It is the one Settings address the navigation gate never refuses. `#/settings` is "This service", for administrators only. The tabs show the service's tabs only to someone who may read them. The shell's part is one line: the rail's Settings opens `#/settings/account`, which is never hidden. | Settings in the menu, opening on "For you" with her email, password and menu layout. No administrators' tabs, and no "You do not have access" page. | A personal preference gated by an administrator's permission. A page that pops entries in or out as permission checks answer. | Purpose: everyone can change their own things. Value: rung 1 never meets a refusal for something that was always hers. |
| B. Keep one Settings page, and replace the service section with a line for non-administrators. | The same, once the shell stops gating `/settings`, which needs a larger change in `NavigationAccess.tsx`. | Same, later. | Same, but nothing she can reach today. |
| C. Move Layout to You. | Layout on her account page beside sessions. | The gate. | Mixes "who I am" with "how the menu sits", and You is already four cards deep. |

Recommendation: **A**. It works today at `#/settings/account` for everyone and needs one href from the shell. It matches CONSENSUS ("the service's own settings apart from my preferences").

Second go. Looked at `#/settings/account` on the stand-in as an administrator and as a non-administrator (`/configuration` refused). The non-administrator sees "Settings / Your sign-in, and how Lys looks in this browser.", one tab "For you", Your account, and Layout, with no refusal anywhere. The administrator sees the same with three more tabs after it. What is still wrong: (1) until the shell points the rail at `#/settings/account`, a non-administrator still has no Settings in the menu, so the page is right and the door to it is missing; that is the ask. (2) A single tab "For you" on its own looks like a stray tab; I kept it, because hiding the tab row for one person and showing it for another moves the page. (3) "Change password" is offered to someone who signs in with Google and has no password; that is item 9, next. (4) The layout is still per browser, and the words now say so; keeping it per person needs a server setting, which I did not ask for because it is polish, not a blocker.

## Item 6. "Lys is running on this computer" on a second computer, and an answer kept per browser

Getting started (`features/dashboard/GettingStarted.tsx`, domain 06's file) asks "Lys is running on this computer. Should your agents run here too?" with the Lys host's own name filled in, whichever device the browser is on. "Not this one" is kept in this browser's localStorage, so it is asked again on every other browser and phone.

| Option | What Cherie sees | Mistake it avoids | Purpose served |
|---|---|---|---|
| A. Name the host by its name: "Lys runs on Cherie's MacBook Pro. Should your agents run there too?", label "That computer's name", and keep "Not this one" on the server for the person (asks to 06 and the server). | On her phone, a question about the computer it is really about, asked once. | "This computer" meaning the server (CONSENSUS: it always means the device in hand; the host is named). | Problem: she is never asked to install agents on a computer she isn't holding. |
| B. Ask only whether this browser's device should run agents, and drop the pre-filled host name. | A question about her phone, which cannot run agents. | The wrong noun, but it asks the wrong question. | Worse: phones and tablets cannot answer yes. |
| C. Words only, answer still per browser. | Right words, asked again on each device. | The wrong noun. | Half. |

Recommendation: **A**. None of it is in my folders: the words are an ask to 06 with the exact lines, and keeping the answer per person is a server ask. Nothing is implemented in this item.

Second go. Re-read the file to check the ask is exact, and found one more "this computer" in its invalid-name message ("Give this computer a name"). It is in the ask too. What is still wrong: nothing changes on screen until 06 applies it. That is the honest state of an item whose every line is in another domain's file. I kept it to an ask rather than reach into the file.

## Item 7. When the Lys computer is off, an open tab says "ask the administrator"

A fresh page cannot load at all; that is the browser's own error, and only on-device Lys or a separate host can change it. A tab already open shows "The service could not complete this request", then "If you were making a change, keep its original request…" even on a plain read, then "Lys could not be reached. Ask the administrator to check the service." For rung 1 the administrator is the person whose computer is off.

| Option | What Cherie sees | Mistake it avoids | Purpose served |
|---|---|---|---|
| A. The page names where and why: "Lys at lys.cullen.design is not answering", "The computer Lys runs on may be off, asleep or off the network. When it answers, this page reads again by itself.", "If it stays like this, check that computer is switched on and awake." The technical reason is kept under Error details. No resubmit warning on a read, and no "Back to the start" link, which would fail the same way. The shared line in `Words.tsx` is an ask. | The real cause, which she can act on: wake the computer. Then the page comes back by itself. | "Ask the administrator" alone; a warning about changes on a page where she made none (CONSENSUS: every error names the real cause and the next act). | Problem: "the second my computer wasn't available it stopped working" becomes a sentence that says which computer and what to do. |
| B. Keep the layout, change only the shared `ServiceUnreachable` words. | Same words, but under the false heading and the resubmit paragraph. | Half the mistake. | Half. |
| C. A shell-wide banner when the feed drops. | A strip across every page. | The scattered errors. | Shell work (01's), and it would duplicate the page's own words. |

Recommendation: **A** in `RefusedPage`, which every page's failed read passes through, plus the ask to make the shared inline line say the same.

Second go. Looked at You on the stand-in with `/api/me` failing the way a switched-off host fails (the fetch itself rejects): "Lys at 127.0.0.1:5187 is not answering", the cause, the act, and Error details folded. Two short paragraphs split the cause from its act with a large gap, so they are now one sentence: "The computer Lys runs on may be off, asleep or off the network: check that it is switched on and awake. When it answers, this page reads again by itself." What is still wrong: (1) the eyebrow above it says "SIGNED IN AS", the page title You passes, which is item 10. (2) Inline errors on every other page still use the shared `ServiceUnreachable` line in `people/Words.tsx`, "…Ask the administrator to check the service." That is an ask, so the surface says one thing, once applied. (3) The feed that brings the page back reconnects on a 250 ms to 5 s timer (`live.ts:48-50`), which CONSENSUS forbids; it is the shell's, and already listed under Fixed waits. (4) A fresh page with the host off is still the browser's own error; only on-device Lys changes that.
