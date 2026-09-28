const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];

/** Seconds since the epoch as the mock-up writes a date, `22 Sep`. */
export const day = (seconds: number): string => {
  const d = new Date(seconds * 1000);
  return `${d.getDate()} ${MONTHS[d.getMonth()]}`;
};

/** A date and the minute, `22 Sep 09:14`. */
export const clock = (seconds: number): string => {
  const d = new Date(seconds * 1000);
  return `${day(seconds)} ${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}`;
};
