import manifest from '../../../docs/estate/approval-plan.json';
import { planOf } from '../src/features/import/EstateApproval';
import { beforeEach, expect, it, vi } from 'vitest';
import { click, mount, text } from './harness';
import { ADA, SERVICE, ok, refused } from './fixtures';

const loader = 'op-' + 'c'.repeat(32);
const plan = { version:1, agents:[{display_name:'Scribe'}], resources:[{kind:'cambium.channel',id:'lys',relation:'participant',actions:['read','send'],seats:['Scribe'],evidence:'fixture evidence'}] };
const routes = {...SERVICE, '/identity/estate-plan':ok({plan,loader})};
const approve = () => [...document.querySelectorAll('button')].find((button) => (button.getAttribute('aria-label') ?? button.textContent) === 'Approve and apply estate grants') ?? null;
// The test checks stable operation reuse, not the browser's SHA implementation.
beforeEach(()=>{vi.stubGlobal('crypto',{randomUUID:()=> 'aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa',subtle:{digest:async()=>new Uint8Array(32).buffer}});});
it('loading previews exact actions and recipients without recording a grant',async()=>{
  const {posted}=await mount('#/access/import',routes);
  expect(posted).toEqual([]);expect(text()).toContain('read, send');expect(text()).toContain('Scribe');
  expect((approve() as HTMLButtonElement).disabled).toBe(false);expect(document.querySelector('input[type=file]')).toBeNull();
});
it('an uncertain root answer stops without a delegation or automatic retry',async()=>{
  const {posted}=await mount('#/access/import',{...routes,'POST /grants/roots':refused(503,'Unavailable','Unknown outcome')});
  await click(approve());
  expect(posted).toHaveLength(1);expect(posted[0].path).toBe('/grants/roots');
  expect(posted[0].body).toMatchObject({holder:ADA,resource:{kind:'cambium.channel',id:'lys'},pass_on:{kind:'to',actions:['read','send'],recipients:['agent','service_account']}});
  expect(text()).toContain('no automatic retry');
  const first=posted[0];await click(approve());expect(posted).toHaveLength(2);expect(posted[1]).toEqual(first);
});
it('an inactive agent is refused before any root is sent',async()=>{
  const {posted}=await mount('#/access/import',{...routes,'/identity/estate-plan':ok({loader,plan:{...plan,agents:[{display_name:'Courier'}],resources:[{...plan.resources[0],seats:['Courier']}]}})});
  await click(approve());expect(posted).toEqual([]);expect(text()).toContain('must be one active agent');
});
it('a mismatched root receipt never authorises the loader delegation',async()=>{
  const {posted}=await mount('#/access/import',{...routes,'POST /grants/roots':ok({operation:'op-wrong',grant:'grant-'+'d'.repeat(32),receipt:{caller:ADA}})});
  await click(approve());
  expect(posted).toHaveLength(1);expect(text()).toContain('did not confirm this operation');
});

it('every resource in the shipped estate manifest satisfies the served token rule',()=>{
  expect(()=>planOf(manifest)).not.toThrow();
  for(const r of manifest.resources) for(const token of [r.kind,r.id,r.relation,...r.actions]) expect(token).toMatch(/^[a-z0-9_.-]{1,128}$/);
});
it('invalid resource ids never reach an approval button or a mutation',async()=>{
  const {posted}=await mount('#/access/import',{...routes,'/identity/estate-plan':ok({loader,plan:{...plan,resources:[{...plan.resources[0],id:'default/src_land'}]}})});
  expect(text()).toContain('Invalid resource token');expect(approve()).toBeNull();expect(posted).toEqual([]);
});

it('approval applies through the installed loader with no file download',async()=>{
  const grant='grant-'+'d'.repeat(32);
  const receipt=(body:unknown)=>ok({operation:(body as {operation:string}).operation,grant,receipt:{caller:ADA}});
  const {posted}=await mount('#/access/import',{...routes,'POST /grants/roots':receipt,'POST /grants':receipt,
    'POST /identity/estate-apply':(body)=>{const doc=body as {agents:{display_name:string}[];delegations:{name:string}[]};return ok({by:{kind:'service_account',id:loader},completed:[...doc.agents.map((a)=>({entry:'agents/'+a.display_name,operation:'op-'+'a'.repeat(32)})),...doc.delegations.map((d)=>({entry:'delegations/'+d.name,operation:'op-'+'b'.repeat(32)}))]});}});
  await click(approve());expect(posted.map((p)=>p.path)).toEqual(['/grants/roots','/grants','/identity/estate-apply']);
  expect(text()).toContain('Estate grants applied: 1 delegations recorded for 1 agents.');
  expect(document.querySelector('a[download]')).toBeNull();
});

it('a failed loader apply cannot be reported as completed or retried automatically',async()=>{
  const receipt=(body:unknown)=>ok({operation:(body as {operation:string}).operation,grant:'grant-'+'d'.repeat(32),receipt:{caller:ADA}});
  const {posted}=await mount('#/access/import',{...routes,'POST /grants/roots':receipt,'POST /grants':receipt,'POST /identity/estate-apply':refused(503,'StorageUncertain','unknown outcome')});
  await click(approve());expect(posted.map((p)=>p.path)).toEqual(['/grants/roots','/grants','/identity/estate-apply']);
  expect(text()).toContain('StorageUncertain');expect(text()).toContain('no automatic retry');expect(text()).not.toContain('Estate grants applied:');
});
