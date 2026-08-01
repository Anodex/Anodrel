# Architecture Decision Records

Important decisions are recorded here using sequential numbers.

Each record should include:

- status;
- date;
- context;
- decision;
- consequences;
- conditions that would cause the decision to be revisited.

Current records (newest first):

- 0017 — Windows Authenticode verification is isolated from launch authority.

- 0016 — The first diagnostic log is typed and host-owned.

- 0015 — The brand mark ships as the authored asset, not a reconstruction.
  Supersedes the asset reasoning in 0013.

- 0014 — Startup Lab shows planned actions in a declared pending state.

- 0013 — First-party surfaces are drawn by an owned software renderer.

- 0012 — Windows host owns per-window state and final-window shutdown.

- 0011 — Windows host uses a bounded single-instance lifecycle.

- 0010 — Application hosting starts with a verified owned text package.

- 0001 â€” Anodrel lives in its own repository.
- 0002 â€” Windows is the first supported operating system.
- 0003 â€” Establish the protocol and mock host before a native host.
- 0004 â€” Tao/Wry Windows proof host (superseded).
- 0005 â€” Production native hosts are first-party modules over OS APIs.
- 0006 â€” First production-path Windows host uses owned Win32 modules.
- 0007 â€” Native transport starts with an owned bounded session engine.
- 0008 â€” Windows transport uses an owned authenticated named pipe.
- 0009 â€” Windows child bootstrap uses a one-use inherited anonymous pipe.
