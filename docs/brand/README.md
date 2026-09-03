# Serenade brand assets (`docs/brand`)

Public named copies and **resize-only** size declensions.

## Source of truth

Masters live under local `assets/` (ChatGPT export filenames). That directory is
**gitignored** and must stay on disk as the crop master set. **Do not delete or
mutate `assets/`.**

This folder holds:

1. Byte-identical renamed masters (`logo-banner.png`, …)
2. Width/size variants (`-readme`, `-desktop`, `-mobile`, `-256`, `-128`, `-64`, favicons)

Processing rule: **copy + LANCZOS resize only**. No flood-fill, no alpha rewriting.

## Export timestamp → name map

| `assets/` time suffix | Master name              |
| --------------------- | ------------------------ |
| `12_45_28`            | `mark-cat-lamp`          |
| `12_45_29`            | `logo-wordmark-glow`     |
| `12_45_30`            | `logo-horizontal-soft`   |
| `12_45_31`            | `logo-banner`            |
| `12_45_32`            | `logo-scene-lantern`     |
| `12_45_33`            | `seal-chat-noir`         |
| `12_45_34`            | `seal-s-cat`             |
| `12_45_35`            | `icon-app-s-cat`         |
| `12_45_36`            | `logo-lockup-wide`       |
| `12_45_37`            | `mark-s-cat`             |
| `12_45_38`            | `mark-cat-sit`           |
| `12_45_39`            | `icon-app-eyes`          |
| `12_45_40`            | `icon-app-s`             |
| `12_45_41`            | `icon-circle-cat`        |
| `12_45_42`            | `icon-circle-s`          |
| `12_45_43`            | `seal-s-ring`            |
| `12_45_44`            | `logo-banner-paris`      |
| `12_58_15`            | `logo-banner-rooftop`    |
| `12_58_16`            | `logo-horizontal-night`  |
| `12_58_17`            | `logo-wordmark-pill`     |
| `12_58_18`            | `logo-banner-eyes`       |
| `12_58_19`            | `seal-s-cat-ring`        |
| `12_58_20`            | `icon-app-s-glow`        |
| `12_58_21`            | `logo-lockup-scene`      |
| `12_58_22`            | `logo-wordmark-bar`      |
| `12_58_23`            | `mark-eyes`              |
| `12_58_24`            | `mark-cat-head`          |
| `12_58_25`            | `icon-circle-s-fill`     |
| `12_58_26`            | `seal-cat-frame`         |

## Suggested defaults

- README header: `logo-banner-eyes-readme.png` + `icon-app-eyes-256.png`
- Desktop header: `logo-banner-desktop.png`
- Mobile header: `logo-banner-mobile.png` or `icon-app-eyes-128.png`
- Favicon: `favicon-32.png`

Tagline: **Rust application framework.**
