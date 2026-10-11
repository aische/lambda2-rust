//! The example programs from the website's dropdown (`ex/0.txt` … `ex/17.txt`).

pub struct Example {
    pub number: u32,
    /// File name without number and extension, e.g. `fibonacci-1`.
    pub slug: &'static str,
    /// Title as shown on the website.
    pub title: &'static str,
    pub source: &'static str,
}

pub const EXAMPLES: &[Example] = &[
    Example {
        number: 0,
        slug: "simple-function-1",
        title: "simple function I",
        source: include_str!("../examples/00-simple-function-1.lam"),
    },
    Example {
        number: 1,
        slug: "simple-function-2",
        title: "simple function II",
        source: include_str!("../examples/01-simple-function-2.lam"),
    },
    Example {
        number: 2,
        slug: "simple-function-3",
        title: "simple function III",
        source: include_str!("../examples/02-simple-function-3.lam"),
    },
    Example {
        number: 3,
        slug: "simple-function-4",
        title: "simple function IV",
        source: include_str!("../examples/03-simple-function-4.lam"),
    },
    Example {
        number: 4,
        slug: "fibonacci-1",
        title: "fibonacci numbers I",
        source: include_str!("../examples/04-fibonacci-1.lam"),
    },
    Example {
        number: 5,
        slug: "fibonacci-2",
        title: "fibonacci numbers II",
        source: include_str!("../examples/05-fibonacci-2.lam"),
    },
    Example {
        number: 6,
        slug: "fibonacci-3",
        title: "fibonacci numbers III",
        source: include_str!("../examples/06-fibonacci-3.lam"),
    },
    Example {
        number: 7,
        slug: "prime-numbers",
        title: "prime numbers",
        source: include_str!("../examples/07-prime-numbers.lam"),
    },
    Example {
        number: 8,
        slug: "quick-sort",
        title: "'quick'-sort",
        source: include_str!("../examples/08-quick-sort.lam"),
    },
    Example {
        number: 9,
        slug: "mutual-recursion",
        title: "mutual recursion",
        source: include_str!("../examples/09-mutual-recursion.lam"),
    },
    Example {
        number: 10,
        slug: "parallel-1",
        title: "parallel evaluation I",
        source: include_str!("../examples/10-parallel-1.lam"),
    },
    Example {
        number: 11,
        slug: "parallel-2",
        title: "parallel evaluation II",
        source: include_str!("../examples/11-parallel-2.lam"),
    },
    Example {
        number: 12,
        slug: "parallel-3",
        title: "parallel evaluation III",
        source: include_str!("../examples/12-parallel-3.lam"),
    },
    Example {
        number: 13,
        slug: "parallel-4",
        title: "parallel evaluation IV",
        source: include_str!("../examples/13-parallel-4.lam"),
    },
    Example {
        number: 14,
        slug: "parallel-5",
        title: "parallel evaluation V",
        source: include_str!("../examples/14-parallel-5.lam"),
    },
    Example {
        number: 15,
        slug: "natural-numbers",
        title: "natural numbers",
        source: include_str!("../examples/15-natural-numbers.lam"),
    },
    Example {
        number: 16,
        slug: "ackermann",
        title: "ackerman function",
        source: include_str!("../examples/16-ackermann.lam"),
    },
    Example {
        number: 17,
        slug: "merge-sort",
        title: "merge sort",
        source: include_str!("../examples/17-merge-sort.lam"),
    },
];

/// Looks an example up by number (`4`), slug (`fibonacci-1`) or file stem (`04-fibonacci-1`).
pub fn find(key: &str) -> Option<&'static Example> {
    let key = key.strip_suffix(".lam").unwrap_or(key);
    EXAMPLES.iter().find(|e| {
        key.parse::<u32>().ok() == Some(e.number) || key == e.slug || key == format!("{:02}-{}", e.number, e.slug)
    })
}
