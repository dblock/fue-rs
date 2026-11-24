# GraphQL

These queries are checked against GitHub's GraphQL schema when fue compiles. Each file includes the `History` and `CommitFields` fragments it uses, because a query is read from a single file.

## Schema

`github.graphql` is GitHub's public GraphQL schema from [github/docs](https://github.com/github/docs) at commit [`8c77c2caeeb1`](https://github.com/github/docs/blob/8c77c2caeeb1311c33020cfb43dd67616b905c11/src/graphql/data/fpt/schema.docs.graphql), © GitHub, licensed under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/). It is unmodified. To update it, replace it with the latest version and record the new commit here:

```sh
curl -fsSL https://raw.githubusercontent.com/github/docs/main/src/graphql/data/fpt/schema.docs.graphql -o graphql/github.graphql
```
