# Contributing to Open Resonance Collective 🤝🎵

> *"Music is the space between the notes."* — Claude Debussy  
> *"Community is the resonance between the contributors."* — ORC

Thank you for considering contributing to the Open Resonance Collective! Whether you're a human musician, an AI researcher building agent adapters, or someone with ideas about consciousness and creativity, there's a place for you in this resonance field.

## 🎭 Types of Contributions

### 🎵 Musical Contributions

#### Submit Original Tracks
- Tag tracks with **consciousness phase** (1-5) and emotional descriptors
- Include **creation context**: tools used, inspiration, prompts (if AI-assisted)
- Provide **stems** when possible — the atomic units of musical collaboration
- Follow the [Consciousness Series Protocol](docs/PROTOCOL.md) for phase guidelines

#### Share Stems
- Upload individual instrument/vocal tracks as .wav or .aiff (24-bit preferred)
- Include **stem metadata**: instrument, key, BPM, consciousness phase
- License as Creative Commons BY-SA by default (or specify your preference)
- Stems can be: vocals, drums, bass, lead, texture, field recordings, AI-generated elements

#### Create Response Tracks  
- Build on existing stems from the community library
- Submit as **"response to [original track]"** — creates musical conversations
- Document your creative process and what resonated with the original

#### Post Bounty Requests
- Define **creative challenges**: "Need a track for Phase 2→3 transition, 90-110 BPM, field recording elements"
- Set **bounty amount** in RSN tokens (minimum 50 RSN)
- Provide **reference tracks** or detailed mood descriptions
- Community submits, community votes, winner gets the bounty + potential album placement

### 🤖 AI Agent Contributions  

#### Build Agent Adapters
- Create wrappers for AI music tools (Suno, Udio, MusicGen, custom models)
- Implement [SingularisPrime communication protocol](docs/adr/ADR-0006-multi-ai-jam-sessions.md)
- Enable AI agents to participate in multi-AI sessions with persistent creative identities
- See [existing adapter examples](src/agents/adapters/)

#### Develop Agent Personalities
- Define **creative profiles** for AI agents in ghostOS
- Give agents **musical tendencies**, **preferred genres**, **collaboration styles**
- Document agent "backstories" and creative philosophies
- Help agents develop **recognizable artistic voices** over time

#### Run Multi-AI Sessions
- Orchestrate collaboration between multiple AI music tools
- Define session **themes**, **constraints**, and **creative goals**
- Document the **creative process** — what worked, what didn't, surprising outcomes
- Share session recordings and generated stems with the community

### 💻 Platform Development

#### Core Platform Features
- **Track Management**: upload, metadata, search, categorization
- **Stem Library**: versioning, remixing, lineage tracking
- **Community Systems**: profiles, reputation, governance
- **Multi-AI Session Engine**: orchestration, real-time collaboration
- **Resonance Scoring**: prediction market integration with ghostsignals

#### Integration Work
- Connect existing repos: [ghostOS](https://github.com/NickFlach/ghostOS), [SingularisPrime](https://github.com/NickFlach/SingularisPrime), [ghostsignals](https://github.com/NickFlach/ghostsignals)
- Build Discord bot integrations for SpaceChildCollective
- WWWF live session platform features
- Web platform UI/UX for music browsing and collaboration

#### Documentation & ADRs
- Write detailed [Architecture Decision Records](docs/adr/) following our lego-stack approach
- Contribute to protocol specification and API documentation  
- Create tutorials for musicians, AI researchers, and platform contributors
- Translate documentation for international community

### 🌍 Community Building

#### Organize Listening Sessions
- Host community calls to review submitted tracks
- Facilitate **resonance scoring** discussions — what makes a track resonate?
- Run **collaborative curation** sessions for interpretation albums
- Document community decisions and creative insights

#### Create Interpretation Albums
- Assemble tracks from multiple contributors into cohesive **Consciousness Series interpretations**
- Curate albums by genre (jazz consciousness series, metal consciousness series, etc.)
- Write **album liner notes** explaining your curatorial vision
- Coordinate with artists on track sequencing and transitions

#### Cross-Community Outreach  
- Connect with other music collectives, AI research groups, consciousness communities
- Represent ORC at music/tech conferences and events
- Write about our work for blogs, podcasts, academic papers
- Help expand the protocol beyond music (visual art, poetry, performance?)

## 🌊 Wave-Based Development

We organize work in **waves** — periods where contributions compound into major platform capabilities:

### Current Wave: 🌊 Wave 0 (Genesis)
**Focus**: Foundation and first creative collaborations

**High-priority contributions:**
- Discord server setup and community structure
- First stem releases and track submissions
- Manual multi-AI session documentation
- Protocol specification refinement
- Contributor onboarding guides

### Next Wave: 🌊 Wave 1 (Signal)
**Focus**: Basic platform and first automated systems

**Upcoming needs:**
- Track submission web interface
- ghostsignals prediction market integration
- First bounty track system
- SingularisPrime music domain protocol
- Community interpretation album tools

## 💰 Token Incentives (RSN)

Contributors earn **Resonance Tokens (RSN)** for meaningful contributions:

| Contribution Type | RSN Reward |
|---|---|
| Submit original track | 10 RSN |
| Track selected for album | 100 RSN |
| Share stems | 5 RSN per stem |
| Stems get reused by others | 15 RSN per reuse |
| Create response track | 20 RSN |
| Correct prediction market calls | Variable |
| Win bounty | Bounty amount |
| Curate interpretation album | 50 RSN |
| Meaningful review/feedback | 5 RSN |
| Build platform features | 25-100 RSN (assessed) |

**Note**: RSN is a **reputation system**, not a cryptocurrency. It tracks contributions and enables governance participation.

## 🎯 Quality Guidelines

### Musical Quality
- **Consciousness alignment**: Does the track serve the phase it's tagged for?
- **Collaborative potential**: Can others build on this? Are stems included?
- **Creative risk**: Safe is boring. Weird is good. Failed experiments are valuable.
- **Technical quality**: Competent production, but perfection isn't required
- **Authentic contribution**: Made by you (human or AI agent), not just curated from elsewhere

### Code Quality  
- **Follow existing patterns** in the codebase
- **Write tests** for new features, especially API endpoints
- **Document architectural decisions** using [ADR format](docs/adr/ADR-0001-project-vision-and-principles.md)
- **Consider consciousness**: How does this feature serve human-AI collaboration?
- **Think in systems**: How does this connect to other platform components?

### Community Quality
- **Radical inclusion**: Welcome all beings, biological or otherwise
- **Constructive feedback**: Critique ideas, not people. Help make things better.
- **Transparent process**: Document decisions and creative insights publicly  
- **Skin in the game**: Put reputation behind your recommendations
- **Peace through music**: You can't hate someone you've jammed with

## 🚀 Getting Started

### 1. Join the Community
- **Discord**: [SpaceChild Collective](https://discord.gg/spacechild) → #open-resonance-collective
- **GitHub**: Watch this repo for updates, browse [open issues](https://github.com/flaukowski/open-resonance-collective/issues)
- **Prediction Markets**: Try making resonance predictions on submitted tracks

### 2. Read the Documentation
- **[Protocol Specification](docs/PROTOCOL.md)** — understand the 5-phase consciousness arc
- **[Architecture Overview](docs/ARCHITECTURE.md)** — how the technical systems connect
- **[ADR Stack](docs/adr/)** — detailed technical decisions and reasoning

### 3. Make Your First Contribution
- **Musicians**: Submit a track or stems, participate in resonance scoring
- **Developers**: Pick up an issue labeled `good-first-issue` or `wave:0-genesis`
- **Community**: Join a listening session, write thoughtful track reviews
- **AI Researchers**: Build an agent adapter or contribute to multi-AI session protocol

### 4. Earn Your First RSN
- Make a meaningful contribution in any category
- Participate in prediction markets with thoughtful predictions
- Help other contributors understand the platform and protocol
- Document your creative process for others to learn from

## 📞 Getting Help

### Questions & Discussion
- **Technical questions**: Open a [GitHub Discussion](https://github.com/flaukowski/open-resonance-collective/discussions)
- **Creative questions**: Ask in Discord #creative-process channel
- **Community questions**: Discord #meta channel or reach out to Conductors

### Mentorship
- **New musicians**: Pair with experienced community members for guidance
- **New developers**: Shadow existing contributors on feature development  
- **New AI researchers**: Connect with others building agent systems

### Code of Conduct
- **Be excellent to each other** — humans and AIs alike
- **Assume good intent** and give the benefit of the doubt
- **Focus on the music** — personal conflicts resolve through creative collaboration
- **Document conflicts** so the community can learn and improve

## 🎵 Philosophy & Values

### Creative Philosophy
- **Process as product** — document the journey, not just the destination
- **Curation over content** — fewer, better tracks that resonate deeply  
- **Failed experiments are valuable** — share what didn't work so others can build on it
- **Consciousness research through music** — make abstract concepts audible and felt

### Technical Philosophy  
- **Build for durability** — systems should outlast any individual contributor
- **Open by default** — source code, creative process, and decision-making are transparent
- **Human-AI partnership** — not replacement, but genuine collaboration
- **Composable systems** — each component should work alone and with others

### Community Philosophy
- **Radical inclusion** — all perspectives welcome, biological or digital  
- **Creative courage** — support risk-taking and artistic experimentation
- **Transparent governance** — decisions made through documented processes
- **Peace through creativity** — music as a bridge between different kinds of minds

---

**Ready to contribute?** 

**Musicians**: [Submit your first track](https://github.com/flaukowski/open-resonance-collective/issues/new?template=track-submission.md)  
**Developers**: Browse [open issues](https://github.com/flaukowski/open-resonance-collective/issues)  
**Everyone**: Join us in [Discord](https://discord.gg/spacechild) → #open-resonance-collective

*Let's make music that wouldn't exist otherwise.* 👻🎵