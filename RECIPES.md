# GH CLI Recipes

Quick reference for `gh` commands to paste into GH Query tiles (omit the leading `gh `).

---

## Pull Requests

### My open PRs across all repos
```
search prs --assignee @me --state open --json number,title,repository,isDraft,updatedAt,url
```

### Open PRs by a specific author
```
search prs "author:nil-scan is:open" --json number,title,repository,isDraft,updatedAt,url
```

### PRs involving me (assigned, mentioned, review requested) excluding my own
```
search prs --involves @me --state open --json number,title,repository,isDraft,updatedAt,url
```

### PRs involving me in a specific repo and org (no author negation — not supported by gh CLI)
```
search prs --involves @me --owner solo-io --repo kgateway-dev/kgateway --state open --json number,title,repository,isDraft,updatedAt,url
```

> **Note:** The `-author:@me` negation from GitHub's web search syntax is not supported by `gh search prs`.
> Omit it and rely on `--involves` to scope results.

### Community PRs vs PRs from my team

Every GH Query tile whose rows include `author` automatically gains an `authorGroup`
column — `team`, `maintainer`, `bot`, or `community` — based on the roster in
**Team** (sidebar, above Settings). Nothing special is needed in the command:

```
pr list --repo kgateway-dev/kgateway --search "created:>{{date:-14d}}" --state open --limit 100 --json author,number,title,labels,state,assignees,createdAt,updatedAt,url
```

Click a badge in the column (or use the filter funnel) to narrow the tile to one
group. Drag the column into place with the **Columns** picker.

### Import the roster from a GitHub team

The **Team** dialog's import sources run this for you, but the same command works
in a tile if you want to eyeball a team's membership:

```
api orgs/kgateway-dev/teams/controller-maintainers/members --paginate --json login
```

Use a bare org to pull everyone: `api orgs/solo-io/members --paginate`.

### Only community PRs (no roster needed)

`-author:` negation works in `--search`, so a tile can exclude your team directly:

```
pr list --repo kgateway-dev/kgateway --search "created:>{{date:-14d}} -author:andy-fong -author:sheidkamp" --state open --limit 100 --json author,number,title,state,createdAt,url
```

> **Note:** multiple *positive* `author:` qualifiers are AND-ed by GitHub search, so
> `author:a author:b` matches nothing. For a team-only tile use the array-variable
> fan-out instead (see "Same command across multiple repos" below, with an
> `{{authors}}` array), or just filter the `authorGroup` column.

---

## Issues

### My open issues across all repos
```
search issues --assignee @me --state open --json number,title,repository,updatedAt,url
```

### Everything involving me (assigned, mentioned, authored, review requested)
```
search issues --involves @me --state open --json number,title,repository,updatedAt,url
```

### Issues I authored across all repos
```
search issues --author @me --state open --json number,title,repository,updatedAt,url
```

### Issues created in the past 7 days in a specific repo
```
issue list --repo OWNER/REPO --search "created:>{{date:-7d}}" --state all --limit 100 --json number,title,state,assignees,updatedAt,url
```

> `{{date:-7d}}` is replaced by the backend with today's date minus 7 days (ISO format). Use any number of days, e.g. `{{date:-30d}}`. `{{today}}` gives today's date.

---

## Milestones

### All issues under a milestone (open + closed)
```
issue list --repo solo-io/gloo-gateway --milestone 2.3 --state all --json number,title,state,assignees,labels,updatedAt,url
```

### All PRs under a milestone (open + closed)
```
pr list --repo solo-io/gloo-gateway --milestone 2.3 --state all --json number,title,state,isDraft,assignees,updatedAt,url
```

> Replace `solo-io/gloo-gateway` and `2.3` with your repo and milestone. `--state all` includes both open and closed — omit it to see only open items.

> **Note:** `gh issue list` and `gh pr list` do not support `repository` in `--json` (it's implicit from `--repo`). The dashboard injects it automatically from the `--repo` flag so the repo column still works for filtering across merged commands. Do not include `repository` in your `--json` fields for these commands.

### Merge issues from two repos under the same milestone
Add two commands to a single tile (use the Edit dialog → Add command):
```
issue list --repo solo-io/gloo-gateway --milestone 2.3 --state all --json number,title,state,assignees,updatedAt,url
issue list --repo kgateway-dev/kgateway --milestone 2.3 --state all --json number,title,state,assignees,updatedAt,url
```
Click a repo cell value to filter to just that repo.

### Same command across multiple repos (using Variables)
Use one command with a `{{repo}}` placeholder and set the Variables field to a JSON object with an array:

**Command:**
```
issue list --repo {{repos}} --milestone 2.3 --state all --json number,title,state,assignees,updatedAt,url
```
**Variables:**
```json
{ "repos": ["solo-io/gloo-gateway", "kgateway-dev/kgateway"] }
```
The tile runs the command once per array element and merges all results. Only one array variable per tile is supported; additional string variables are substituted directly.

---

## Tips

- All `gh search` commands work **cross-repo** with no `--repo` flag needed.
- `--repo` and `--owner` narrow scope; omit them for global searches.
- Add `--limit N` (default 30) to fetch more results.
- The `url` field is used by the tile to make the `number` column a clickable link — always include it.
- Useful `--json` fields: `number`, `title`, `state`, `repository`, `author`, `assignees`, `isDraft`, `updatedAt`, `url`, `labels`, `commentsCount`.
- `gh pr list --json` has **no** `authorAssociation` field (only `gh api` and GraphQL expose it) — use the `authorGroup` column instead.
- Multiple positive `author:` qualifiers in `--search` are AND-ed and match nothing; only `-author:` negation composes.
