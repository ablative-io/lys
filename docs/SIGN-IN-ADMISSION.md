# Sign-in admission

Password sign-ins and provider consent use independent in-memory pools. Each
pool holds at most 1024 flows, with 16 per client network. Entries expire ten
minutes after creation, checked on insert/use. Expiry never runs a timer.
Password attempts allow ten per minute; setup-code entry points share five per
minute. A refused attempt does not extend its fixed window. Restarting clears
these in-memory limits; issuer-side controls still apply.

A client network is an IPv4 address or an IPv6 /64. IPv4-mapped IPv6 is treated
as IPv4. A provider callback must carry the HTTP-only, SameSite=Lax, host-only
cookie set at the start; only its digest is held with the flow. HTTPS callbacks
use a Secure cookie. Starting another provider flow in the same browser replaces
that cookie, so finish one provider sign-in at a time.

## Front proxies

By default only the connection peer supplies the client address. Forwarded
headers are ignored. For a front proxy, explicitly set `trusted_proxies` in the
directory service's startup JSON to its exact IP addresses, for example
`["127.0.0.1", "::1"]` only when those addresses exclusively identify your proxy.
No address or subnet is implicitly trusted. This is a separate setting from
`issuer.trusted_proxies` in the deployment TOML: that setting controls the
issuer's trust in the directory service.

The front proxy must append the actual peer address it accepted to
`X-Forwarded-For`. Lys walks the header from right to left, skipping configured
trusted proxies and using the first untrusted address. Missing or malformed
trusted hops refuse sign-in. Arbitrary left entries cannot override a client's
address. If a proxy is not configured, all its clients share its peer's quota.
Do not trust loopback if untrusted local processes can call the service directly.

Install and upgrade omit this key unless the operator already set it. Upgrade
preserves an explicitly configured list. Enable it only outside a reversible
upgrade: older service versions reject unknown configuration keys. There is no
managed setting-write endpoint; editing a startup file is an operator action.
Rollback restores the saved previous configuration, including its exact bytes.
