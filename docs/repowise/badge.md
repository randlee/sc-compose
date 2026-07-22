repowise health — 
/Users/randlee/Documents/github/sc-compose-worktrees/repowise-analysis
2026-07-21 22:42:32 [info     ] FileTraverser initialised      extra_exclude_patterns=0 include_nested_repos=False max_file_size_kb=500 repo_root=/Users/randlee/Documents/github/sc-compose-worktrees/repowise-analysis submodules_skipped=0
2026-07-21 22:42:32 [debug    ] Skipping oversized file        path=.repowise/duplication_cache.pkl size_kb=3706
2026-07-21 22:42:32 [debug    ] Skipping oversized file        path=.repowise/knowledge-graph.json size_kb=584
2026-07-21 22:42:32 [debug    ] Skipping oversized file        path=.repowise/parse_cache.pkl size_kb=1489
2026-07-21 22:42:32 [debug    ] Skipping oversized file        path=.repowise/wiki.db size_kb=7620
2026-07-21 22:42:32 [debug    ] Built Cargo workspace index    crate_count=3
2026-07-21 22:42:32 [info     ] import_resolution_per_language seconds={'toml': 0.0, 'json': 0.0, 'markdown': 0.0, 'yaml': 0.0, 'python': 0.0, 'rust': 0.01, 'shell': 0.0}
2026-07-21 22:42:32 [info     ] Heritage edges resolved        total=14
2026-07-21 22:42:32 [debug    ] Built Cargo workspace index    crate_count=3
2026-07-21 22:42:32 [info     ] Call edges resolved            total=2065
2026-07-21 22:42:32 [info     ] Graph built                    edge_types={'defines': 2291, 'extends': 7, 'has_method': 502, 'imports': 483, 'calls': 2065, 'implements': 7} edges=5355 file_nodes=323 symbol_nodes=2291
2026-07-21 22:42:33 [debug    ] repo_commit_index_built        commits_parsed=295 files_with_history=82 indexable_files=83
2026-07-21 22:42:33 [debug    ] co_change_computed             commit_limit=2000 commits=295 files_with_entropy=210 files_with_partners=100 min_count=2 pairs_above_threshold=437 pairs_considered=6269 tracked_files=299
2026-07-21 22:42:34 [info     ] Git indexing complete          duration=1.1s files=83 hotspots=16 stable=1 tier=full
2026-07-21 22:42:34 [debug    ] duplication_token_cache        hits=263 misses=3
Static badge (current score):
  !(https://img.shields.io/badge/health-8.2%2F10-brightgreen)

Live badge (running Repowise server or hosted repo):
  !(https://img.shields.io/endpoint?url=<SERVER>/api/repos/<REPO_ID>/health/badg
e.json)
