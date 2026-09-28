import type {ReactNode} from 'react';
import clsx from 'clsx';
import Link from '@docusaurus/Link';
import useDocusaurusContext from '@docusaurus/useDocusaurusContext';
import Layout from '@theme/Layout';
import CodeBlock from '@theme/CodeBlock';
import Heading from '@theme/Heading';
import HomepageFeatures from '@site/src/components/HomepageFeatures';
import styles from './index.module.css';

type PanelProps = {
  ordinal: string;
  label: string;
  title: string;
  chip?: ReactNode;
  alt?: boolean;
  children: ReactNode;
};

function PanelSection({ordinal, label, title, chip, alt, children}: PanelProps) {
  return (
    <section className={clsx(styles.section, alt && styles.sectionAlt)}>
      <div className={styles.sectionInner}>
        <div className={styles.panel}>
          <div className={styles.panelHeader}>
            <div className={styles.panelEyebrow}>
              <span className={styles.panelOrdinal}>{ordinal}</span>
              <span>{label}</span>
            </div>
            {chip ? <span className={styles.chip}>{chip}</span> : null}
          </div>
          <Heading as="h2" className={styles.panelTitle}>
            {title}
          </Heading>
          <div className={styles.panelBody}>{children}</div>
        </div>
      </div>
    </section>
  );
}

function HomepageHeader() {
  const {siteConfig} = useDocusaurusContext();
  return (
    <header className={clsx('hero', styles.heroBanner)}>
      <div className={styles.heroInner}>
        <Heading as="h1" className="hero__title">
          {siteConfig.title}
        </Heading>
        <p className="hero__subtitle">{siteConfig.tagline}</p>
        <div className={styles.buttons}>
          <Link className="button button--primary button--lg" to="/docs/getting-started">
            Quickstart
          </Link>
          <Link className="button button--secondary button--lg" to="/docs">
            What this is
          </Link>
        </div>
      </div>
    </header>
  );
}

function TheProblem() {
  return (
    <PanelSection ordinal="01" label="The problem" title="Prose can be followed plausibly and still be wrong">
      <p>
        An instruction such as “write the test first, preserve compatibility, and obtain approval
        before production changes” reads well and enforces nothing. An agent can produce an answer
        that sounds compliant without leaving the facts the rule depends on.
      </p>
      <CodeBlock language="text">
        {`"The story is done."   → an assertion
$ aep plan artifact move story:pay-by-card --to implemented
story:pay-by-card is active; implemented is on the ladder and not yet earned:
reaching implemented needs at least 1 test_result record(s). …`}
      </CodeBlock>
      <p>
        AEP turns the operative parts into validated data and leaves reasoning to the model. A
        status is decided from the evidence the store actually holds.
      </p>
    </PanelSection>
  );
}

function TheClaim() {
  return (
    <PanelSection ordinal="03" label="The claim" title="Completion is a decision over evidence" alt>
      <p>
        Lifecycles declare legal progress and what each rung costs. Evidence records say who observed
        what, when, and about which artifact. The decision is deterministic: a legal move, or a
        refusal that names what is missing. The plan itself is plain files in your repository, and
        Git is its history.
      </p>
      <CodeBlock language="yaml">
        {`kind: story
initial: draft
transitions:
  draft: [proposed, archived]
  proposed: [draft, active, rejected]
  active: [implemented, archived]
requires:
  implemented:
    - evidence: test_result
      at_least: 1`}
      </CodeBlock>
      <p className={styles.panelMore}>
        <Link to="/docs/concepts/lifecycles">How lifecycles and rungs work →</Link>
        {' · '}
        <Link to="/docs/concepts/evidence">Evidence and completion →</Link>
      </p>
    </PanelSection>
  );
}

function HonestStatus() {
  return (
    <PanelSection
      ordinal="04"
      label="Status"
      title="The protocol is separate from models and plugins"
      chip={
        /* generated:release-chip:begin — do not edit; run `cargo xtask status` */
        <code>0.64.0</code>
        /* generated:release-chip:end */
      }>
      <div className={styles.ledger}>
        <p className={styles.ledgerBuilt}>
          AEP ships the Git-native planning store, the governed-task engine, the reference driver,
          the trace checker, the published schemas, and the <code>aep</code> command.
        </p>
        <p className={styles.ledgerNot}>
          Executable system modeling lives in ESS. Harness-specific skills and agents live in the
          curated agentplugins marketplace. AEP selects neither and accepts them only at explicit
          adapter boundaries.
        </p>
      </div>
      <p className={styles.panelMore}>
        <Link to="/docs/status/where-this-stands">Where this stands →</Link>
        {' · '}
        <Link to="/docs/status/limitations">Limitations and trust assumptions →</Link>
      </p>
    </PanelSection>
  );
}

export default function Home(): ReactNode {
  const {siteConfig} = useDocusaurusContext();
  return (
    <Layout title="Agentic engineering under machine-checkable constraints" description={siteConfig.tagline as string}>
      <HomepageHeader />
      <main>
        <TheProblem />
        <HomepageFeatures />
        <TheClaim />
        <HonestStatus />
      </main>
    </Layout>
  );
}
