repowise health — 
/Users/randlee/Documents/github/sc-compose-worktrees/repowise-analysis
2026-07-24 10:15:08 [info     ] FileTraverser initialised      extra_exclude_patterns=0 include_nested_repos=False max_file_size_kb=500 repo_root=/Users/randlee/Documents/github/sc-compose-worktrees/repowise-analysis submodules_skipped=0
2026-07-24 10:15:08 [debug    ] Skipping oversized file        path=.repowise/duplication_cache.pkl size_kb=4496
2026-07-24 10:15:08 [debug    ] Skipping oversized file        path=.repowise/knowledge-graph.json size_kb=707
2026-07-24 10:15:08 [debug    ] Skipping oversized file        path=.repowise/parse_cache.pkl size_kb=1766
2026-07-24 10:15:08 [debug    ] Skipping oversized file        path=.repowise/wiki.db size_kb=11052
2026-07-24 10:15:09 [debug    ] Built Cargo workspace index    crate_count=3
2026-07-24 10:15:09 [info     ] import_resolution_per_language seconds={'toml': 0.0, 'json': 0.0, 'markdown': 0.0, 'yaml': 0.0, 'python': 0.0, 'rust': 0.01, 'shell': 0.0}
2026-07-24 10:15:09 [info     ] Heritage edges resolved        total=15
2026-07-24 10:15:09 [debug    ] Built Cargo workspace index    crate_count=3
2026-07-24 10:15:09 [info     ] Call edges resolved            total=2568
2026-07-24 10:15:09 [info     ] Graph built                    edge_types={'defines': 2637, 'extends': 7, 'has_method': 602, 'imports': 544, 'calls': 2568, 'implements': 8} edges=6366 file_nodes=346 symbol_nodes=2637
2026-07-24 10:15:09 [debug    ] repo_commit_index_built        commits_parsed=340 files_with_history=90 indexable_files=91
2026-07-24 10:15:10 [debug    ] co_change_computed             commit_limit=2000 commits=340 files_with_entropy=241 files_with_partners=113 min_count=2 pairs_above_threshold=496 pairs_considered=9364 tracked_files=402
2026-07-24 10:15:10 [info     ] Git indexing complete          duration=1.3s files=91 hotspots=18 stable=0 tier=full
2026-07-24 10:15:11 [debug    ] duplication_token_cache        hits=285 misses=1
Static badge (current score):
  !(https://img.shields.io/badge/health-7.9%2F10-yellow)

Live badge (running Repowise server or hosted repo):
  !(https://img.shields.io/endpoint?url=<SERVER>/api/repos/<REPO_ID>/health/badg
e.json)
