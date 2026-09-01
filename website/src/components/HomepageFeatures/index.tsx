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
    title: 'AEP',
    governs: 'the shared substrate',
    question: 'What do the recorded facts permit?',
    description: (
      <>
        Typed artifacts, lifecycles, evidence, permissions, approvals, audit and completion. A
        harness asks what is owed and what is permitted; the deterministic answer can always say
        why.
      </>
    ),
    href: '/docs/concepts/aep',
  },
  {
    title: 'ADP',
    governs: 'development work',
    question: 'Was this software change built properly?',
    description: (
      <>
        Specification, decomposition, design, tests, implementation and review expressed as a
        profile over the generic protocol rather than a second planning system.
      </>
    ),
    href: '/docs/examples/governed-task',
  },
  {
    title: 'AOP',
    governs: 'operational work',
    question: 'May this controlled change proceed?',
    description: (
      <>
        Operational planning, approvals, verification, rollback and incidents use the same evidence
        and audit substrate with operations-specific vocabulary.
      </>
    ),
    href: '/docs/concepts/lifecycles',
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
          One substrate, two profiles
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
