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

- **UTM parameters**: Added support for UTM parameters. You can filter and search by UTM source, medium, campaign, content, and term.

### 🐛 Bug Fixes

- Fixed a potential panic when entities are not found in the database (https://github.com/explodingcamera/liwan/commit/31405a721dc5c5493098e211927281cca7816fec)
- Fixed issues with the `Yesterday` Date Range (https://github.com/explodingcamera/liwan/commit/76278b579c5fe1557bf1c184542ed6ed2aba57cd)

## **Liwan v0.1.1** - 2024-09-24

### ⚡ Performance

- **Database indexes**: Removed all unnecessary indexes to improve performance and reduce disk usage (https://github.com/explodingcamera/liwan/commit/6191a72f08e8659237bc6c22139bde94432f66bb)

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
