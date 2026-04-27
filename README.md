# Altheia Identity Program

*Anchor program for the Altheia trust layer — agent identity registry + audit Merkle anchors on Solana.*

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Anchor 0.31](https://img.shields.io/badge/Anchor-0.31-purple.svg)](https://www.anchor-lang.com)
[![Solana 2.0](https://img.shields.io/badge/Solana-2.0-9945FF.svg)](https://solana.com)

## What this is

The on-chain piece of [Altheia](https://altheia.xyz) — the trust + audit layer for AI agents on Solana. This program is a **slim identity registry**: it stores agent identities, policy commitment hashes, and periodic Merkle roots of the off-chain audit log. It does **not** enforce token transfers or hold session keys — that work happens at the [Swig](https://github.com/anagrambuild/swig-wallet) substrate layer, where on-chain scope (per-mint caps, allowed programs, instant revocation) is enforced.

See [SMART_CONTRACT_SRS.md](https://github.com/altheia-xyz/altheia-plan/blob/main/02_SRS/SMART_CONTRACT_SRS.md) for the full specification.

## Account model

| Account | PDA seeds | Purpose |
|---|---|---|
| `OperatorAccount` | `["operator", wallet_pubkey]` | One per operator — holds agent counts, last audit Merkle root |
| `AgentAccount` | `["agent", operator, agent_id]` | One per agent — operator link, framework, policy commitment, Swig account reference, status |

## Instructions (planned)

- [x] `initialize_operator` — create OperatorAccount PDA on first interaction
- [x] `register_agent` — register a new agent (stores policy commitment hash + Swig account reference)
- [ ] `update_policy_commitment` — update an agent's policy hash
- [ ] `pause_agent` / `unpause_agent` — reversible state transitions
- [ ] `revoke_agent` — permanent revocation
- [ ] `archive_agent` — soft-delete (only from Revoked)
- [ ] `commit_audit_root` — periodic Merkle root anchor of the off-chain audit log

Status as of v0.1: scaffold + first two instructions. Remaining instructions land in subsequent commits.

## Local dev (Docker, no host toolchain needed)

The Rust + Solana + Anchor toolchain runs entirely in Docker. You don't need any of it installed locally.

```bash
make image            # one-time: build the Docker image (~10-15 min first time)
make build            # anchor build (compiles the program inside Docker)
make test             # anchor test (spins solana-test-validator in Docker, runs integration tests)
make validator        # run a long-running validator on localhost:8899
make stop             # stop the validator
make shell            # drop into the toolchain shell for ad-hoc commands
make idl              # emit IDL JSON after build
make deploy-devnet    # deploy to Solana devnet (requires ~/.config/solana/id.json with devnet SOL)
make help             # list everything
```

The first `make image` build is heavy (multi-GB of Rust + Solana toolchain). After that, builds are cached in named Docker volumes (`cargo-cache`, `target-cache`).

## Toolchain pins

| Tool | Version |
|---|---|
| Anchor | 0.31.1 |
| Solana CLI | 2.0.20 |
| Rust | 1.79 (in Docker) |
| Node | 22 (for tests) |

Pinned in [Anchor.toml](Anchor.toml) and [Dockerfile](Dockerfile).

## Program ID

Placeholder ID in this scaffold: `AthIdentity1111111111111111111111111111111`. Real ID gets generated on first deploy via `anchor build` (which writes the program keypair to `target/deploy/`). Update [Anchor.toml](Anchor.toml) and `declare_id!` in [lib.rs](programs/identity/src/lib.rs) once you have the real ID.

## Repo layout

```
altheia-program/
├── Anchor.toml              # Anchor config (toolchain pins, cluster, program IDs)
├── Cargo.toml               # workspace manifest
├── Dockerfile               # Rust + Solana + Anchor + Node
├── docker-compose.yml       # anchor + validator services
├── Makefile                 # build / test / deploy targets
├── package.json             # JS test deps (mocha + chai + @coral-xyz/anchor)
├── tsconfig.json
├── programs/
│   └── identity/
│       ├── Cargo.toml
│       └── src/lib.rs       # the program
├── tests/
│   └── identity.ts          # Anchor integration tests
├── migrations/              # Anchor deploy scripts
├── target/                  # build output (gitignored)
└── .github/workflows/ci.yml # CI: build + lint
```

## License

Apache 2.0 — see [LICENSE](LICENSE).

## Related repos

- [altheia-sdk](https://github.com/altheia-xyz/altheia-sdk) — TypeScript SDK + MCP server + framework adapters
- [altheia-plan](https://github.com/altheia-xyz/altheia-plan) — product specs + design + roadmap (private during build phase)
