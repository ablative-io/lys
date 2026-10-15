/** The name people see, made from a person's first and last names as they typed them. */
export function fullName(given: string, family: string): string {
  return `${given.trim()} ${family.trim()}`;
}
