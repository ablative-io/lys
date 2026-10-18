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
