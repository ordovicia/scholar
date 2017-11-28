# scholar [![Build Status](https://travis-ci.org/ordovicia/scholar.svg?branch=master)](https://travis-ci.org/ordovicia/scholar)

Google Scholar scraper written in Rust.

```
USAGE:
    scholar [FLAGS] [OPTIONS]

FLAGS:
    -t, --title-only    Search only papers which contain specified words in their title
        --json          Output in JSON format
    -h, --help          Prints help information
    -V, --version       Prints version information

OPTIONS:
    -c, --count <count>                Maximum number of results
    -w, --words <words>                Search papers with these words
    -p, --phrase <phrase>              Search papers with this exact phrase
    -a, --authors <authors>            Search papers with these authors
        --search-html <search-html>    HTML file of search results
        --cite-html <cite-html>        HTML file of citers list
```

## Note

If you send requests too frequently, Google Scholar will block your access temporarily.
I will not offer any workaround for this situation.

Currently, this program outputs nothing when blocked.

## Related Projects

* Inspired by [ckreibich/scholar.py](https://github.com/ckreibich/scholar.py)
