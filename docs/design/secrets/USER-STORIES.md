# Secrets — User Stories

## Reviewer — Reviews a brief before any of its rows is dispatched

**S1.** As the reviewer, I want the secrets broker's implementation brief in the design-system form, with numbered requirements and criteria, so that its rows can be reviewed and dispatched one at a time.

## Person — Grants an agent provisioned under them access to an account

**S2.** As a person granting an agent access, I want to give it a handle under my own grant, limited by uses, time and spend, so that it can use the account without ever holding the credential and the grant traces back to me.

**S3.** As a person, I want to drop an agent's handle and have every new call on it refused at once, so that taking access back does not wait on rotating the real key.

## AI Agent — Uses a handle for its outbound calls and reads its sealed records

**S4.** As an agent, I want to make my call with my handle and have the proxy swap in the credential, refreshing an expired OAuth token itself, so that I can do my work without ever seeing a credential.

**S5.** As an agent, I want to ask for one of my sealed memories by name and get back only the piece I am permitted, so that a secret never sits in plain text in my memory files.

## Token revolver — The worker that runs Claude sessions and builders and turns to the next account on usage-limit words

**S6.** As the token revolver, when a session prints its usage-limit words, I want to ask the broker for my next account instead of walking my own list, so that the spread across accounts is the broker's and no pool file is copied to my machine.

## Engine — Whichever runtime starts and ends a seat

**S7.** As the engine starting a seat, I want the door to give me the seat's own login from the store at spawn and record which token went to which seat, so that I read no pool file when the broker is there and still work from my own when it is not.

## Operator — Keeps the accounts and revokes access

**S8.** As an operator, I want to rest an account by one change in the store, so that no worker gives it another call and no file is copied to any machine.

**S9.** As an operator revoking access, I want each case to say what it reaches: a handle refuses new calls, a call already admitted follows the stated cancellation rule, a login token's seat is ended by its engine, and read knowledge stays read, so that I am never told revocation took back what it cannot.

## Reviewer — Reads the audit of what the broker did

**S10.** As a reviewer reading the audit, I want one line per use naming the seat, the handle, the real account and the time, and one per sealed read, so that every use is attributed to an identity in one place.

**S11.** As a reviewer reading the audit, I want each call that was in flight when its handle was dropped to show the outcome the cancellation rule gave it, so that no call's end is unaccounted for.
