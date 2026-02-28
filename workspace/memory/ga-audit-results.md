# Google Analytics Audit Results

**Date:** 2026-02-12
**Scope:** All repos in `C:\Users\nickf\Source` (~130 repos)

## Summary

Found Google Analytics tracking in **11 repos**, all using the NickFlach GitHub account. Two GA measurement IDs were in use:
- `G-CMEBRPNPGG` — used across 10 repos (Space Child ecosystem + related projects)
- `G-YWW6FV2SGN` — used in claude-code-templates docs (12 HTML files)

**No Facebook Pixel, Hotjar, or Microsoft Clarity tracking was found.**

## Repos Cleaned

| Repo | Files Changed | Type | Action |
|------|--------------|------|--------|
| angel-informant | index.html | Simple gtag script | Removed |
| cosmic-empathy-core | index.html | Simple gtag script | Removed |
| FlaukowskiFashion | client/index.html | Simple gtag script | Removed |
| FlaukowskiMind | client/index.html | Simple gtag script | Removed |
| ninja-craft-hub | index.html | gtag script + dns-prefetch | Removed |
| Space-Child-Dream | client/index.html | Simple gtag script | Removed |
| space-child-learn | index.html | Simple gtag script | Removed |
| SpaceChild | client/index.html | Simple gtag script | Removed |
| SpaceChildCollective | client/index.html | Extended gtag with trackEvent/trackPageView/trackEngagement + PWA install tracking | Removed GA, kept no-op stubs for window.trackEvent/trackPageView/trackEngagement so existing code doesn't break |
| SpaceChildWaitlist | client/index.html + client/src/lib/analytics.ts | gtag script + full analytics.ts module (initGA, trackPageView, trackEvent) | Removed GA from index.html; replaced analytics.ts functions with no-ops |
| claude-code-templates | 12 files in docs/*.html | Simple gtag script (different ID: G-YWW6FV2SGN) | Removed from all 12 HTML files |

## Commit Details

All repos committed with: `chore: remove Google Analytics tracking for privacy`
All pushed to origin/main.

## False Positives Investigated & Skipped

- **collaborateESCO** — `setToStringTag` in minified JS matched `gtag` pattern; not actual GA
- **agentic-qe** — test report HTML mentioning CSP errors referencing googletagmanager; not actual tracking
- **0xSCADA** — various files with `_ga` in unrelated contexts (CSS classes, variable names); no actual GA tracking
- **agent-zero** — minified transformers.js library; not actual GA

## Privacy Docs Found

- `C:\Users\nickf\Source\MusicPortal\PRIVACY_POLICY.md` — only privacy policy file found across all repos
- No PRIVACY.md files found in the GA-affected repos (could be added as a follow-up)

## Recommendations

1. **Add PRIVACY.md** to Space Child repos stating the no-tracking policy
2. **Consider Plausible/Umami** if aggregate traffic data is still wanted (privacy-preserving, no cookies)
3. **Deactivate the GA properties** G-CMEBRPNPGG and G-YWW6FV2SGN in Google Analytics console to stop data collection entirely
4. **Check deployed sites** — if any are deployed to Replit/Vercel/etc., redeploy to ensure the GA removal goes live
