//! Equality of already-admitted suite/4 definitions inherited by coverage suite/5.
//!
//! This is a comparison view, never a wire-admission route or a replacement for original bytes.
//! Defaults and Node values follow their declared ESS scenario owners. Ordered metadata and u64
//! fields remain exact; arbitrary payload object members are never treated as optional fields.
use std::collections::BTreeMap;

use aep_domain::{Node, Predicate};
use serde::Deserialize;

use crate::count_json::{Json, Result};

pub(super) struct Definitions {
    pub provenance: Provenance,
    pub scenarios: BTreeMap<String, Scenario>,
}

impl Definitions {
    // Called only after the complete source has passed the existing closed admission checks.
    pub fn read(value: &Json) -> Result<Self> {
        fn decode<T: serde::de::DeserializeOwned>(value: &Json) -> Result<T> {
            serde_json::from_str(&value.raw)
                .map_err(|error| value.error("InvalidShape", error.to_string()))
        }
        let fields = value.object()?;
        Ok(Self {
            provenance: decode(&fields["provenance"])?,
            scenarios: decode(&fields["scenarios"])?,
        })
    }
}

#[derive(Deserialize, PartialEq, Eq)]
pub(super) struct Provenance {
    suite_version: String,
    system: String,
    specification_version: String,
    spec_digest: String,
    contract_digest: String,
    #[serde(default)]
    component: Option<String>,
}

#[derive(Deserialize, PartialEq, Eq)]
pub(super) struct Scenario {
    purpose: String,
    steps: Vec<Step>,
    // Keep complete dependency occurrence/order metadata as the coverage contract requires.
    source: Vec<serde_json::Value>,
}

type Values = BTreeMap<String, ScenarioValue>;
type Payload = BTreeMap<String, Node>;
type Shape = BTreeMap<String, LeafShape>;

#[derive(Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum ScenarioValue {
    Literal { value: Node },
    Instance { instance: String },
    Observed { event: String, field: String },
}

#[derive(Deserialize, PartialEq, Eq)]
struct LeafShape {
    #[serde(flatten)]
    holds: Holds,
    #[serde(default)]
    optional: bool,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(tag = "holds", rename_all = "snake_case")]
enum Holds {
    Primitive { kind: String },
    Enum { variants: Vec<String> },
    List,
    Map,
    Union,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(tag = "step", rename_all = "snake_case")]
enum Step {
    ConfigureExternalOutcome {
        force: Outcome,
    },
    ExecuteCommand {
        command: String,
        #[serde(default)]
        actor: Option<String>,
        #[serde(default)]
        input: Values,
    },
    ExpectOutcome {
        outcome: Outcome,
    },
    ExpectError {
        error: String,
        #[serde(default)]
        fields: Payload,
    },
    ExpectEvent {
        event: String,
        #[serde(default)]
        payload: Payload,
        #[serde(default)]
        shape: Shape,
    },
    ExpectNoEvent {
        event: String,
    },
    RedeliverEvent {
        event: String,
    },
    CaptureInstance {
        instance: String,
        entity: String,
        event: String,
        field: String,
    },
    ExpectInvocation {
        binding: String,
        command: String,
        #[serde(default)]
        input: Values,
    },
    QueryView {
        view: String,
        #[serde(default)]
        params: Values,
    },
    ExpectView {
        view: String,
        expectation: Expectation,
    },
    EventuallyEvent {
        event: String,
        #[serde(default)]
        payload: Payload,
        #[serde(default)]
        shape: Shape,
    },
    EventuallyView {
        view: String,
        #[serde(default)]
        params: Values,
        expectation: Expectation,
    },
    MarkInstant {
        instant: String,
    },
    ExpectNotBefore {
        instant: String,
        elapsed: u64,
    },
    ExpectWithin {
        instant: String,
        elapsed: u64,
    },
    ExpectQuiet {
        event: String,
        instant: String,
        elapsed: u64,
    },
    ExpectHalt {
        view: String,
        #[serde(default)]
        params: Values,
        after: u64,
    },
    EventuallyHalt {
        view: String,
        #[serde(default)]
        params: Values,
        after: u64,
    },
}

#[derive(Deserialize, PartialEq, Eq)]
struct Outcome {
    command: String,
    outcome: String,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(tag = "expect", rename_all = "snake_case")]
enum Expectation {
    Contains {
        fields: Values,
    },
    Excludes {
        fields: Values,
    },
    Satisfies {
        predicate: Condition,
    },
    Counts {
        #[serde(default)]
        at_least: Option<u64>,
        #[serde(default)]
        at_most: Option<u64>,
    },
    Ranked {
        order_by: Vec<String>,
    },
    At {
        order_by: Vec<String>,
        position: Position,
        #[serde(default)]
        fields: Values,
    },
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(tag = "row", rename_all = "snake_case")]
enum Position {
    First,
    Last,
    Nth { index: u64 },
}

// ESS extends the shared predicate with quantifiers. Preserve their binders and ordered bodies,
// while delegating scalar operands and their Number semantics to the same inherited parser.
#[derive(PartialEq, Eq)]
enum Condition {
    Leaf(Predicate),
    All(Vec<Self>),
    Any(Vec<Self>),
    Not(Box<Self>),
    Quantified {
        universal: bool,
        over: String,
        bind: String,
        body: Box<Self>,
    },
}

impl<'de> Deserialize<'de> for Condition {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let node = Node::deserialize(d)?;
        Self::read(&node).map_err(serde::de::Error::custom)
    }
}

impl Condition {
    fn read(node: &Node) -> std::result::Result<Self, String> {
        match node {
            Node::Seq(values) => Self::children(values.iter(), true),
            Node::Map(fields) => {
                let children = fields
                    .iter()
                    .map(|(key, value)| Self::entry(key, value))
                    .collect::<std::result::Result<_, _>>()?;
                Ok(Self::combine(children, true))
            }
            _ => Predicate::from_node(node)
                .map(Self::from_predicate)
                .map_err(|error| error.to_string()),
        }
    }

    fn entry(key: &str, value: &Node) -> std::result::Result<Self, String> {
        match key {
            "all" | "and" | "all_of" => Self::children(value.as_seq_or_single(), true),
            "any" | "or" => Self::children(value.as_seq_or_single(), false),
            "none" | "none_of_these" => {
                Self::children(value.as_seq_or_single(), false).map(Self::negate)
            }
            "not" => Self::read(value).map(Self::negate),
            "forall" | "exists" => {
                let fields = value.as_map().ok_or("admitted quantifier is a mapping")?;
                let text = |key| {
                    fields
                        .get(key)
                        .and_then(Node::as_text)
                        .map(str::to_owned)
                        .ok_or("admitted quantifier names collection and binder")
                };
                Ok(Self::Quantified {
                    universal: key == "forall",
                    over: text("in")?,
                    bind: text("as")?,
                    body: Box::new(Self::read(
                        fields.get("that").ok_or("admitted quantifier has a body")?,
                    )?),
                })
            }
            _ => Predicate::from_node(&Node::Map(
                [(key.to_owned(), value.clone())].into_iter().collect(),
            ))
            .map(Self::from_predicate)
            .map_err(|error| error.to_string()),
        }
    }

    fn children<'a>(
        values: impl IntoIterator<Item = &'a Node>,
        all: bool,
    ) -> std::result::Result<Self, String> {
        values
            .into_iter()
            .map(Self::read)
            .collect::<std::result::Result<_, _>>()
            .map(|values| Self::combine(values, all))
    }

    fn combine(mut values: Vec<Self>, all: bool) -> Self {
        match values.len() {
            0 => Self::Leaf(if all {
                Predicate::Always
            } else {
                Predicate::Never
            }),
            1 => values.remove(0),
            _ if all => Self::All(values),
            _ => Self::Any(values),
        }
    }

    fn negate(value: Self) -> Self {
        match value {
            Self::Leaf(Predicate::Always) => Self::Leaf(Predicate::Never),
            Self::Leaf(Predicate::Never) => Self::Leaf(Predicate::Always),
            Self::Not(inner) => *inner,
            other => Self::Not(Box::new(other)),
        }
    }

    fn from_predicate(value: Predicate) -> Self {
        match value {
            Predicate::All(values) => {
                Self::All(values.into_iter().map(Self::from_predicate).collect())
            }
            Predicate::Any(values) => {
                Self::Any(values.into_iter().map(Self::from_predicate).collect())
            }
            Predicate::Not(inner) => Self::Not(Box::new(Self::from_predicate(*inner))),
            other => Self::Leaf(other),
        }
    }
}
