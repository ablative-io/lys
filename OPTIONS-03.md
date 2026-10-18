# Domain 03 (access and permissions): options, item by item

Each item from the Top ten of `03-access-permissions.md`, in order. For each: two or three options, what Cherie (rung 1, first MacBook this weekend) would see, which mistake from the reviews it avoids, which of the page's purposes it serves (what purpose, what problem, what value), my recommendation, then a second go after looking at it.

The page's purposes, said once so the options can point at them:
- **Purpose:** a person can ask for what they need, and the person who can say yes can say yes, without learning a rights model.
- **Problem:** today the way to ask is hidden behind reads that have nothing to do with asking, the words change from tab to tab, and a give leaves the person unsure it happened.
- **Value:** "make the right thing easy, attractive and obvious first" (Tom, 18 Oct 11:02). The right thing here is asking and answering.

## 1. The way to ask is hidden unless four unrelated reads succeed

The rail and the direct entry treat Requests, Reviews and Drafts as "Access", so `/me`, `/people`, `/grants` and `/grants/model` must all read before Requests opens. Inside the page, Requests also fails whole if `/people` is refused, and the Ask form fails whole if `/grants` is refused. The gate itself is in `shell/NavigationAccess.tsx`, which is domain 01's file.

| Option | What Cherie sees | Mistake it avoids | Purpose served |
|---|---|---|---|
| A. Write the shell ask only; leave Requests as it is | Nothing changes until domain 01 applies it; if `/people` is refused she still lands on "Lys could not show your access requests" with no Ask button | Avoids touching another domain's file | Problem, half: the rail; the page still fails whole |
| B. Requests opens on `/requests` alone. Names, teams and her own access are extras: each one missing says so in one line and the page carries on. The Ask form needs only who she is and the list of what can be asked; if her own access cannot be read she gets the "not in the list" fields straight away, with the reason. Plus the shell ask | The queue and the Ask button, always, once she is signed in. A missing extra is one sentence ("People's names could not be read, so requests show who asked by the name they gave") instead of a dead end | "The person who most needs to ask is the one who cannot reach the Ask button" (03, Wrong refusals 1 and 2); "a refusal that stops people using it is the failure that cannot happen" (Tom, 13 Oct) | Purpose (ask is always there), problem (no unrelated read can hide it) |
| C. B, and also a stand-alone ask page that needs no queue at all | Same as B, with a second address for the same form | Same as B | Same, at the cost of a second copy of the form: the duplication mistake (03 "Duplication", four copies of retained-request code) |

**Recommendation: B.** It is the whole fix inside my folders, and the shell ask closes the rest. C adds a copy, which is the mistake the review counts four of already.

**Ask written** in CONSENSUS.md under 03: give Requests, Reviews and Drafts their own entry reads in `NavigationAccess.tsx`, and keep the person-state short cut away from them.

**Second go.** Reading the rendered page as Cherie would, my first wording was wrong in a way the tests could not see: both new lines ended with the service's stock advice, "Ask the directory administrator to make the change", which sends her to someone over a list she does not need, and in the Ask form it sat right beside "Enter what you need below", two next steps that pull opposite ways. A missing extra is not her problem to fix, so it now says its cause and no chore ("People's names are not shown here, so each request shows who asked by the name they gave. Lys does not show this account that list."), in the quiet note style, not the red alert one. What is still wrong: the form's own fields still say "Type of thing" and "Its name" and the access select still reads "reader · read"; those are items 3 and 5, not this one. And until domain 01 applies the shell ask, the rail and the direct entry still hide this page behind the four reads, so for a person whose `/grants` is refused nothing visible changes yet; the page is ready for it, and a test proves the page side against the old code (two of its three cases fail on e36b08d7).
