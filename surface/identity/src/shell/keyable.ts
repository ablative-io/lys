import type { KeyboardEvent } from 'react';

// Everything clickable that is not a link or a button is focusable and answers
// Enter and Space, as the mock-up's KEYABLE rule makes it (conformance 9.3).
export const keyable = (activate: () => void) => ({
  tabIndex: 0,
  role: 'button' as const,
  onKeyDown: (event: KeyboardEvent) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      event.stopPropagation();
      activate();
    }
  },
});
