import type { KeyboardEvent } from 'react';

// Everything clickable that is not a link or a button is focusable and answers
// Enter and Space, as the mock-up's KEYABLE rule makes it (conformance 9.3).
// The key dispatches one click on the element itself, so the action runs once,
// through the same onClick a pointer reaches, and any click listener sees it.
export const keyable = (activate: () => void) => ({
  onClick: activate,
  tabIndex: 0,
  role: 'button' as const,
  onKeyDown: (event: KeyboardEvent) => {
    // Only a key pressed on the element itself, as the mock-up matches e.target:
    // a link or button inside it keeps its own Enter.
    if (event.target !== event.currentTarget) return;
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      event.stopPropagation();
      event.currentTarget.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
    }
  },
});
