Please name all migrations after this pattern:

```
{last_version}-{migration_index}-{description}.sql
```

where

- `last_version` denotes the version this migration is built _on_, not for (e.g.
  if you write a migration when knb is at version `1.0.1` but the migration will
  be in effect in `1.0.2`, use `1.0.1`)
- `migration_index` says at which position this migration should be applied
- `description` consists of a few words combined in kebab-case, saying what this
  migration does
