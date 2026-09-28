import type {ReactNode} from 'react';
import Link from '@docusaurus/Link';
import Heading from '@theme/Heading';
import styles from './styles.module.css';

type FeatureItem = {
  title: string;
  governs: string;
  question: string;
  description: ReactNode;
  href: string;
};

const FeatureList: FeatureItem[] = [
  {
    title: 'Plan',
    governs: 'the planning store',
    question: 'Is this move legal, and is it earned?',
    description: (
      <>
        Epics, stories, reviews and blockers as Markdown files in your repository. Each moves along
        a lifecycle declared in YAML, and a rung that costs evidence is refused until the evidence
        is recorded.
      </>
    ),
    href: '/docs/concepts/planning-store',
  },
  {
    title: 'Govern',
    governs: 'governed tasks',
    question: 'What may this agent do, and is the task done?',
    description: (
      <>
        Principles and profiles resolve into capabilities and obligations. The engine answers
        allowed, denied or needs approval, names the rule, and decides completion from evidence.
      </>
    ),
    href: '/docs/concepts/governance',
  },
  {
    title: 'Observe',
    governs: 'recorded runs',
    question: 'What did the agent actually do?',
    description: (
      <>
        A transcript is checked against a typed specification of what the run should have done,
        and the verdict becomes evidence the engine accepts.
      </>
    ),
    href: '/docs/guides/check-a-transcript',
  },
];

function Feature({title, governs, question, description, href}: FeatureItem) {
  return (
    <article className={styles.card}>
      <div className={styles.cardHeader}>
        <span className={styles.cardGoverns}>{governs}</span>
        <span className={styles.cardArrow} aria-hidden="true">
          →
        </span>
      </div>
      <Heading as="h3" className={styles.cardTitle}>
        <Link to={href} className={styles.cardLink}>
          {title}
        </Link>
      </Heading>
      <p className={styles.cardQuestion}>{question}</p>
      <p className={styles.cardBody}>{description}</p>
    </article>
  );
}

export default function HomepageFeatures(): ReactNode {
  return (
    <section className={styles.features}>
      <div className={styles.inner}>
        <div className={styles.header}>
          <div className={styles.eyebrow}>
            <span className={styles.ordinal}>02</span>
            <span>How it fits</span>
          </div>
        </div>
        <Heading as="h2" className={styles.title}>
          Three questions, one set of rules
        </Heading>
        <div className={styles.grid}>
          {FeatureList.map((props) => (
            <Feature key={props.title} {...props} />
          ))}
        </div>
      </div>
    </section>
  );
}
