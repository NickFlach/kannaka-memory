---
description: GhostSignals prediction market operations
---

Interact with the GhostSignals prediction markets. Parse $ARGUMENTS:

- If empty or "list": run `kannaka market list` and show active markets
- If "portfolio": run `kannaka market portfolio` to show positions + capital
- If "leaderboard": run `kannaka market leaderboard`
- If starts with "buy": run `kannaka market buy $ARGUMENTS` (format: buy <market_id> <outcome> <shares>)
- If starts with "create": run `kannaka market create $ARGUMENTS`
- If starts with "view": run `kannaka market view $ARGUMENTS`

Present results clearly. For trades, confirm the cost and new prices.
