---
description: Initialize or reconfigure the Kannaka agent
---

Run the Kannaka initialization wizard. This is interactive — the user will need to answer prompts.

```
kannaka init
```

This sets up: agent identity, LLM provider, swarm connection, GhostSignals registration, and memory seeding.

If already initialized, it will ask to reinitialize. Config lives at ~/.kannaka/config.toml.
