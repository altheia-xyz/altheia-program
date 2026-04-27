// Anchor deploy script (runs after `anchor deploy`).
// Use this for any one-off post-deploy initialization (e.g., creating a
// canonical authority account on devnet).

import * as anchor from "@coral-xyz/anchor";

module.exports = async function (provider: anchor.AnchorProvider) {
  anchor.setProvider(provider);
  // No-op for v0.1 — operators initialize themselves via `initialize_operator`.
  console.log("deploy.ts: nothing to migrate. operators self-initialize.");
};
