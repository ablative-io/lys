/** The bare buttons still standing outside the act control (shell/Act.tsx), file by file, counted from the source on
 *  6 October 2026. A number here goes down in the commit that moves those buttons to the control, and its line goes
 *  when it reaches none. Nothing is added and no number is raised: a new act is an `Act`. */
export const STANDING: Record<string, number> = {
  'features/access/Access.tsx': 1,
  'features/access/Graph.tsx': 1,
  'features/apps/Apps.tsx': 1,
  'features/apps/SchemaBuilder.tsx': 1,
  'features/connections/SignInProviders.tsx': 1,
  'features/dashboard/AgentTree.tsx': 5,
  'features/drafts/Drafts.tsx': 1,
  'features/file/HeadMenu.tsx': 1,
  'features/file/sections.tsx': 1,
  'features/network/AddMachine.tsx': 1,
  'features/network/Network.tsx': 2,
  'features/people/People.tsx': 1,
  'features/provisioning/FolderChooser.tsx': 1,
  'features/requests/Requests.tsx': 1,
  'features/runtime/CanvasDock.tsx': 7,
  'features/runtime/CanvasMarks.tsx': 4,
  'features/runtime/CanvasTogether.tsx': 4,
  'features/runtime/CanvasWidgets.tsx': 7,
  'features/runtime/SessionCanvas.tsx': 7,
  'features/runtime/Terminal.tsx': 3,
  'features/settings/Settings.tsx': 4,
  'features/setup/Setup.tsx': 3,
  'features/sign-in/SignIn.tsx': 1,
  'features/teams/Teams.tsx': 1,
  'shell/Explain.tsx': 1,
  'shell/Listing.tsx': 3,
  'shell/Picker.tsx': 2,
  'shell/Rail.tsx': 1,
};

/** How many stood when the count began. The sum above never passes it. */
export const STOOD_AT_THE_START = 246;
