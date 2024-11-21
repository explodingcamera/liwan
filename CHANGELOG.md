# Changelog

All notable changes to this project will be documented in this file.

<!--
The format is roughly based on the output of `git-cliff` and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

- Types of changes
  - `⚠️ Breaking Changes`
  - `🚀 Features`
  - `🐛 Bug Fixes`
  - `📚 Documentation`
  - `⚡ Performance`
  - `🛡️ Security`

Since this is not a library, this changelog focuses on the changes that are relevant to the end-users. For a detailed list of changes, see the commit history, which adheres to [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/). New releases are created automatically when a new tag is pushed (Commit message: chore(release): vX.X.X).
-->

## [Unreleased]

### 🚀 Features

- **UTM parameters**: Added support for UTM parameters. You can filter and search by UTM source, medium, campaign, content, and term. ([#13](https://github.com/explodingcamera/liwan/pull/13))
- **New Date Ranges**: Fully reworked date ranges. Data is more accurate and consistent now, and you can move to the next or previous time range. Also includes some new time ranges like `Week to Date` and `All Time`. You can now also select a custom date range to view your data.
- **UI Improvements**: A lot of small improvements to the UI for better polish and usability.
- **New Metrics**: Added new metrics: `Bounce Rate`, `Average Time on Page`

### 🐛 Bug Fixes

- Added more tests that fixed some smaller bugs
- Fixed a potential panic when entities are not found in the database ([`31405a72`](https://github.com/explodingcamera/liwan/commit/31405a721dc5c5493098e211927281cca7816fec))
- Fixed issues with the `Yesterday` Date Range ([`76278b57`](https://github.com/explodingcamera/liwan/commit/76278b579c5fe1557bf1c184542ed6ed2aba57cd))

### Other

- Removed Sessions and Average Views per Session metrics. They were not accurate and were removed to avoid confusion.

## **Liwan v0.1.1** - 2024-09-24

### ⚡ Performance

- **Database indexes**: Removed all unnecessary indexes to improve performance and reduce disk usage ([`6191a72f`](https://github.com/explodingcamera/liwan/commit/6191a72f08e8659237bc6c22139bde94432f66bb))

## **Liwan v0.1.0** - 2024-09-18

This is the first full release of the Liwan! 🎉
All essential features for web analytics are now available, including:

- Live tracking of page views
- Geolocation of visitors
- Automatic GeoIP database updates
- Basic user management
- Filtering and searching
- Multiple tracking dimensions: URL, referrer, browser, OS, device type, country, and city
- Multiple metrics: page views, unique visitors, sessions, and average views per session
- Multiple date ranges (custom date ranges are coming soon!)
- Documentation and a simple setup guide at [liwan.dev](https://liwan.dev)
- A simple and clean UI
