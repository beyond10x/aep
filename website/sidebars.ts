import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

/**
 * One hand-written sidebar, ordered the way a first-time reader adopts the tool: what it is and a
 * first run, the model behind it, task-oriented guides, the reference, a worked example, and where
 * the project stands.
 */
const sidebars: SidebarsConfig = {
  docsSidebar: [
    {
      type: 'category',
      label: 'Start here',
      collapsed: false,
      items: ['index', 'getting-started'],
    },
    {
      type: 'category',
      label: 'Concepts',
      collapsed: false,
      items: [
        'concepts/overview',
        'concepts/artifacts',
        'concepts/lifecycles',
        'concepts/evidence',
        'concepts/planning-store',
        'concepts/reviews',
        'concepts/waves',
        'concepts/workspaces',
        'concepts/governance',
        'concepts/design-principles',
      ],
    },
    {
      type: 'category',
      label: 'Guides',
      collapsed: false,
      items: [
        'guides/plan-work',
        'guides/gate-a-move-on-evidence',
        'guides/review-with-findings',
        'guides/run-waves',
        'guides/migrate-an-older-store',
        'guides/validate-in-ci',
        'guides/govern-a-task',
        'guides/write-a-principle',
        'guides/integrate-a-harness',
        'guides/check-a-transcript',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: false,
      items: [
        'reference/cli',
        'reference/project-file',
        'reference/planning-stores',
        'reference/artifact-file',
        'reference/evidence-file',
        'reference/lifecycle-file',
        'reference/documents',
        'reference/vocabulary',
        'reference/harnesses',
        'reference/glossary',
      ],
    },
    {
      type: 'category',
      label: 'Examples',
      collapsed: false,
      items: ['examples/governed-task'],
    },
    {
      type: 'category',
      label: 'Project status',
      collapsed: false,
      items: ['status/where-this-stands', 'status/limitations', 'status/roadmap'],
    },
  ],
};

export default sidebars;
