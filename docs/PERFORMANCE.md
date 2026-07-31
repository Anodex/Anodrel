# Anodrel Performance Baseline

Anodrel will be considered better than Electron only where measurements support
the claim. Electron embeds Chromium and Node.js and uses a browser-style
multi-process model; it also advises developers to profile their specific apps,
avoid unnecessary modules, and keep work off the UI process. Anodrel will use
those concerns as testable design requirements, not as a claim that every
Anodrel application is automatically faster.

## Non-negotiable design properties

| Area | Anodrel requirement | Current protection |
| --- | --- | --- |
| Runtime ownership | No third-party shipped native runtime dependency. | Native dependency tree contains only Anodrel crates. |
| Native work | Keep raw OS calls isolated behind small adapters. | Win32 calls live only in `native/hosts/windows/src/win32.rs`. |
| Message memory | Bound bytes before parsing and bound a single receive burst. | 64 KiB payload; four framed messages per receive. |
| Startup | Do no application I/O, network work, or deferred-service initialization before the first window is responsive. | Current host performs only its internal health check. |
| UI responsiveness | Never block the Windows message loop on stream I/O or expensive work. | Required by the transport contract; adapter work is not implemented yet. |
| Privilege | Native authority remains host-issued and operation-specific. | The transport frame carries no authority; core ignores supplied capability context. |

## Measurements before comparison claims

Before claiming an improvement over an Electron version of the same application,
record equivalent release builds on the same machine and workload:

1. cold-start time to first responsive window;
2. private bytes, working set, and idle CPU after 30 seconds;
3. 50th/95th/99th percentile request latency for 1 KiB and 64 KiB messages;
4. frame throughput and allocation behavior under fragmented and coalesced I/O;
5. UI responsiveness while a long-running native operation executes; and
6. installed package size, update size, and number of shipped runtime
   components.

Store hardware, OS build, workload, sample count, and raw results with every
comparison. A result is only meaningful for the named application and workload.

## Initial verification

`cargo tree --manifest-path native/Cargo.toml` verifies the owned native graph.
The wire and session unit tests verify framing, limits, fragmentation,
coalescing, authentication, and capability policy without timing-sensitive
assertions. The Windows adapter integration test exercises a real local named
pipe from connection through authenticated health response. Repeatable runtime
measurement commands will be added before any performance comparison is
published.

## Reference material

- Electron process model: <https://www.electronjs.org/docs/latest/tutorial/process-model>
- Electron security guidance: <https://www.electronjs.org/docs/latest/tutorial/security>
- Electron performance guidance: <https://www.electronjs.org/docs/latest/tutorial/performance>
