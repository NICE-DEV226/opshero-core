# W9 rerun (python3 -I for all parsing; read-only gh; secondary rate limits hit: scripts sleep/retry)
cd this dir
./discover.sh                  # gh search prs, 5 agent signatures x 10 weekly slices (cap 40 each) -> discover/
python3 -I select_repos.py     # stars via GraphQL; seeded sample of repos (all >100-star + 160 random <=100) -> repos_selected.json
python3 -I fetch_prs.py        # per repo: discovered agent PRs + 40 newest PRs in window -> raw/
python3 -I build_sample.py     # classify (classify.py) -> prs.json, sample.json
python3 -I fetch_ci.py         # check suites + 7d default-branch history (STOPPED EARLY: 62/220 repos) -> ci/
python3 -I analyze.py          # -> results.md, tables/pr_rows.csv
