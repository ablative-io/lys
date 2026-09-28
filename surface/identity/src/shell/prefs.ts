// Shell preferences, kept under the same keys the mock-up uses.

export const pref = (key: string, fallback: string): string => {
  try {
    return localStorage.getItem('iam.' + key) || fallback;
  } catch {
    return fallback;
  }
};

export const setPref = (key: string, value: string): void => {
  try {
    localStorage.setItem('iam.' + key, value);
  } catch {
    // A browser that refuses storage keeps the preference for this page only.
  }
};
