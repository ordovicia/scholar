use std::io;
use std::ops::Deref;

use select::document::Document;
use select::node::Node;
use select::predicate::{Attr, Class, Name, Predicate, Text};

use paper::Paper;
use errors::*;

macro_rules! try_html {
    ($a: expr) => { $a.ok_or(ErrorKind::BadHtml)? }
}

pub struct SearchDocument(Document);
pub struct CitationDocument(SearchDocument);

impl Deref for SearchDocument {
    type Target = Document;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a> From<&'a str> for SearchDocument {
    fn from(s: &str) -> Self {
        let document = Document::from(s);
        SearchDocument(document)
    }
}

impl SearchDocument {
    pub fn new(document: Document) -> Self {
        SearchDocument(document)
    }

    pub fn from_read<R: io::Read>(readable: R) -> Result<Self> {
        let document = Document::from_read(readable)?;
        Ok(Self::new(document))
    }

    pub fn scrape_papers(&self) -> Result<Vec<Paper>> {
        // <div id="gs_res_ccl_mid">
        //   <div class="gs_ri">
        //     paper
        //   </div>
        //   ...
        // </div>

        let pos = Attr("id", "gs_res_ccl_mid").descendant(Class("gs_ri"));
        let nodes = self.find(pos);

        let mut papers = Vec::with_capacity(10);
        for n in nodes {
            papers.push(Self::scrape_paper_one(&n)?);
        }

        Ok(papers)
    }

    fn scrape_paper_one(node: &Node) -> Result<Paper> {
        let title = Self::scrape_title(node);
        let (id, c) = Self::scrape_id_and_citation(node)?;

        Ok(Paper {
            title,
            id,
            citation_count: Some(c),
            citers: None,
        })
    }

    fn scrape_title(node: &Node) -> String {
        // There are (at least) two formats.
        //
        // 1. Link to a paper or something:
        //
        // <h3 class="gs_rt">
        //   <span>
        //       something
        //   </span>
        //   <a href="http://paper.pdf">
        //     Title of paper or something
        //   </a>
        // </h3>
        //
        // 'span' may not exists.
        //
        // 2. Not a link:
        //
        // <h3 class="gs_rt">
        //   <span>
        //       something
        //   </span>
        //   Title of paper or something
        // </h3>

        // 1. Link to a paper or something
        let pos = Class("gs_rt").child(Name("a"));
        if let Some(n) = node.find(pos).nth(0) {
            return n.text();
        }

        // 2. Not a link
        let children = node.find(Class("gs_rt")).into_selection().children();
        let text_nodes = children.filter(|n: &Node| {
            if let Some(name) = n.name() {
                name != "span"
            } else {
                true
            }
        });
        let concated_text = text_nodes
            .into_iter()
            .map(|n| n.text())
            .collect::<String>()
            .trim()
            .to_string();
        concated_text
    }

    // Scrape article footer for
    //
    // * cluster id, and
    // * citation count
    fn scrape_id_and_citation(node: &Node) -> Result<(u64, u32)> {
        // Footer format:
        //
        // <div class="gs_fl">
        //   (something)
        //   <a href="/scholar?cites=000000>Cited by 999</a>
        //   (something)
        // </div>

        let pos = Class("gs_fl");
        let footers = try_html!(node.find(pos).nth(0)).children();

        let citation_node = footers
            .into_selection()
            .filter(|n: &Node| {
                if let Some(id_url) = n.attr("href") {
                    parse_id_from_url(id_url).is_ok()
                } else {
                    false
                }
            })
            .first();
        let citation_node = try_html!(citation_node);

        let id = {
            let id_url = citation_node.attr("href").unwrap();
            parse_id_from_url(id_url).unwrap()
        };

        let citation_count = parse_citation_count(&citation_node.text())?;

        Ok((id, citation_count))
    }
}

impl Deref for CitationDocument {
    type Target = SearchDocument;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl CitationDocument {
    pub fn new(document: Document) -> Self {
        CitationDocument(SearchDocument::new(document))
    }

    pub fn from_read<R: io::Read>(readable: R) -> Result<Self> {
        let document = Document::from_read(readable)?;
        Ok(Self::new(document))
    }

    pub fn scrape_target_paper(&self) -> Result<Paper> {
        let node = {
            let pos = Attr("id", "gs_rt_hdr")
                .child(Name("h2"))
                .child(Name("a").or(Text));
            try_html!(self.find(pos).nth(0))
        };

        let title = node.text();
        let id = {
            let id_url = try_html!(node.attr("href"));
            parse_id_from_url(id_url)?
        };

        Ok(Paper {
            title,
            id,
            citation_count: None,
            citers: None,
        })
    }

    pub fn scrape_target_paper_with_citers(&self) -> Result<Paper> {
        let target_paper = self.scrape_target_paper()?;
        let citers = self.scrape_papers()?;

        Ok(Paper {
            citers: Some(citers),
            ..target_paper
        })
    }
}

fn parse_id_from_url(url: &str) -> Result<u64> {
    use regex::Regex;

    lazy_static! {
        static ref RE: Regex = Regex::new(r"(cluster|cites)=(\d+)").unwrap();
    }

    let caps = try_html!(RE.captures(url));
    let id = {
        let id = try_html!(caps.get(2));
        id.as_str().parse()?
    };

    Ok(id)
}

fn parse_citation_count(text: &str) -> Result<u32> {
    use regex::Regex;

    lazy_static! {
        static ref RE: Regex = Regex::new(r"[^\d]+(\d+)").unwrap();
    }

    let caps = try_html!(RE.captures(text));
    let count = {
        let count = try_html!(caps.get(1));
        count.as_str().parse().unwrap()
    };

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_id_from_url_pass() {
        assert_eq!(parse_id_from_url("cluster=123456").unwrap(), 123456);
        assert_eq!(parse_id_from_url("scholar?cluster=654321").unwrap(), 654321);
        assert_eq!(
            parse_id_from_url("scholar?cluster=222222&foo=bar").unwrap(),
            222222
        );
    }

    #[test]
    fn parse_id_from_url_fail() {
        assert!(parse_id_from_url("foo").is_err());
        assert!(parse_id_from_url("claster=000000").is_err());
        assert!(parse_id_from_url("cluster=aaaaaa").is_err());
    }

    #[test]
    fn parse_citation_count_pass() {
        assert_eq!(parse_citation_count("Cited by 111").unwrap(), 111);
        assert_eq!(parse_citation_count("引用元 222").unwrap(), 222);
    }

    #[test]
    fn parse_citation_count_fail() {
        assert!(parse_citation_count("foo").is_err());
    }

    #[test]
    fn search_document_scrape_test() {
        use std::fs;

        let papers = {
            let file = fs::File::open("src/test_html/quantum_theory.html").unwrap();
            let doc = SearchDocument::from_read(file).unwrap();
            doc.scrape_papers().unwrap()
        };

        assert_eq!(papers.len(), 10);

        assert_eq!(
            papers[0],
            Paper {
                title: String::from("Quantum field theory and critical phenomena"),
                id: 16499695044466828447,
                citation_count: Some(4821),
                citers: None,
            }
        );

        assert_eq!(
            papers[1],
            Paper {
                title: String::from("Quantum theory of solids"),
                id: 8552492368061991976,
                citation_count: Some(4190),
                citers: None,
            }
        );

        assert_eq!(
            papers[2],
            Paper {
                title: String::from(
                    "Significance of electromagnetic potentials in the quantum theory"
                ),
                id: 5545735591029960915,
                citation_count: Some(6961),
                citers: None,
            }
        );
    }

    #[test]
    fn citation_document_scrape_test() {
        use std::fs;

        let doc = {
            let file = fs::File::open("src/test_html/quantum_theory_citations.html").unwrap();
            CitationDocument::from_read(file).unwrap()
        };

        let target_paper = doc.scrape_target_paper().unwrap();
        let citer_papers = doc.scrape_papers().unwrap();

        assert_eq!(
            target_paper,
            Paper {
                title: String::from(
                    "Significance of electromagnetic potentials in the quantum theory"
                ),
                id: 5545735591029960915,
                citation_count: None,
                citers: None,
            }
        );

        assert_eq!(citer_papers.len(), 10);

        assert_eq!(
            citer_papers[0],
            Paper {
                title: String::from("Quantal phase factors accompanying adiabatic changes"),
                id: 15570691018430890829,
                citation_count: Some(7813),
                citers: None,
            }
        );

        assert_eq!(
            citer_papers[1],
            Paper {
                title: String::from("Multiferroics: a magnetic twist for ferroelectricity"),
                id: 9328505180409005573,
                citation_count: Some(3232),
                citers: None,
            }
        );

        assert_eq!(
            citer_papers[2],
            Paper {
                title: String::from("Quantum field theory"),
                id: 14398189842493937255,
                citation_count: Some(2911),
                citers: None,
            }
        );
    }
}
