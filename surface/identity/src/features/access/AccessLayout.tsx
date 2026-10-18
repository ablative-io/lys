/**
 * Every Access view's frame: the one PageHeader titled Access, the view's purpose in a line, its actions, then the
 * tabs, then the view's own content. Only the content waits on a read: the head and tabs are drawn at once and stay
 * when a read is refused, so a person who lands on a refusal is one press from every other view. The head and tabs
 * are direct children of the page, which is how the shared page styles place them.
 */
import type { ReactNode } from 'react';
import { PageHeader } from '../../shell/PageHeader';
import { AccessTabs } from './AccessTabs';

export function AccessLayout({ on, subtitle, actions, className, children }: {
  /** The tab this view is, by its key in `ACCESS_TABS`. */
  on: string; subtitle: ReactNode; actions?: ReactNode; className?: string; children: ReactNode;
}) {
  return <div className={'page fill' + (className ? ' ' + className : '')}>
    <PageHeader title="Access" subtitle={subtitle} actions={actions} />
    <AccessTabs on={on} />
    {children}
  </div>;
}
