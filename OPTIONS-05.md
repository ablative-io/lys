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
