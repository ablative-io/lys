import { expect, it } from 'vitest';
import { click, mount, text } from './harness';
import { SERVICE, ok, refused } from './fixtures';

const secret='f'.repeat(64);
const BACK='https://notes.example.test/signed-in';
// The registration asks for one return address; approval keeps it as the sign-in settings, the name withheld.
const pending={id:'fixture_notes',name:'Notes fixture',state:'pending',redirects:[BACK],sign_in:null,schema:{kinds:{}},version:0,versions:[],pending:null,client_id:null,service_account:null,registered_by:{kind:'start'},registered_at:1};
const approvedApp={...pending,state:'approved',sign_in:{redirects:[BACK],profile:false,operation:'op-approve',by:{kind:'start'},at:2}};
const button=(label:string)=>[...document.querySelectorAll('button')].find((b)=>(b.getAttribute('aria-label')??b.textContent)===label)??null;
function routes(){let approved=false;return {...SERVICE,'/apps':()=>ok({apps:[approved?approvedApp:pending]}),
  'POST /apps/fixture_notes/approve':()=>{approved=true;return ok({app:approvedApp,client:{client_id:'fixture_notes',client_secret:secret,credential:'lys-app.fixture_notes.'+secret}});}};}
it('saves without copying and removes credential values after broker confirmation',async()=>{
  const {posted}=await mount('#/apps',{...routes(),'POST /apps/fixture_notes/credentials/save':ok({app:'fixture_notes',client_secret_ref:'sealed-client',api_credential_ref:'sealed-api'})});
  await click(button('Approve Notes fixture'));await click(button('Save credentials in Lys secrets'));
  expect(posted[0]).toEqual({path:'/apps/fixture_notes/approve',body:{operation:expect.stringMatching(/^op-/),redirects:[BACK],profile:false}});
  expect(posted[1]).toEqual({path:'/apps/fixture_notes/credentials/save',body:{client_secret:secret}});
  expect(text()).toContain('Credentials saved in Lys secrets');expect(text()).not.toContain(secret);
  expect(JSON.stringify(Object.entries(sessionStorage))).not.toContain(secret);expect(JSON.stringify(Object.entries(localStorage))).not.toContain(secret);
});
it('a refused save keeps the issued credential only in memory and makes no automatic retry',async()=>{
  const {posted}=await mount('#/apps',{...routes(),'POST /apps/fixture_notes/credentials/save':refused(503,'SecretsUnavailable','unknown outcome')});
  await click(button('Approve Notes fixture'));await click(button('Save credentials in Lys secrets'));
  expect(posted).toHaveLength(2);expect(text()).toContain('The save was not confirmed');expect(text()).toContain('unknown outcome');expect(text()).toContain('Nothing is tried again on its own');expect(text()).not.toContain('Credentials saved');
  await click(button('Save credentials in Lys secrets'));expect(posted).toHaveLength(3);expect(posted[2]).toEqual(posted[1]);
});
it('a malformed success never clears the only issued copy or claims saved',async()=>{
  await mount('#/apps',{...routes(),'POST /apps/fixture_notes/credentials/save':ok({app:'another-app',client_secret_ref:'x',api_credential_ref:'y'})});
  await click(button('Approve Notes fixture'));await click(button('Save credentials in Lys secrets'));
  expect(text()).toContain('did not confirm');expect(text()).not.toContain('Credentials saved');expect(button('Save credentials in Lys secrets')).not.toBeNull();
});

it('broker-backed approval shows saved references without a second save or plaintext',async()=>{
  const {posted}=await mount('#/apps',{...routes(),
    'POST /apps/fixture_notes/approve':ok({app:approvedApp,client:null,credentials:{app:'fixture_notes',client_secret_ref:'sealed-client',api_credential_ref:'sealed-api'}})});
  await click(button('Approve Notes fixture'));
  expect(posted).toHaveLength(1);
  expect(text()).toContain('Credentials saved in Lys secrets');
  expect(text()).not.toContain(secret);
  expect(button('Save credentials in Lys secrets')).toBeNull();
});
it('refused custody leaves approval available and never retries automatically',async()=>{
  const {posted}=await mount('#/apps',{...routes(),
    'POST /apps/fixture_notes/approve':refused(502,'SecretsUnavailable','custody not confirmed')});
  await click(button('Approve Notes fixture'));
  expect(posted).toHaveLength(1);
  expect(text()).toContain('SecretsUnavailable');
  expect(text()).not.toContain('Credentials saved');
  expect(button('Approve Notes fixture')).not.toBeNull();
});
it('never puts an issued secret value on the screen, not even behind a toggle',async()=>{
  await mount('#/apps',{...routes(),'POST /apps/fixture_notes/credentials/save':refused(503,'SecretsUnavailable','unknown outcome')});
  await click(button('Approve Notes fixture'));
  for(const toggle of document.querySelectorAll('details'))toggle.open=true;
  expect(document.body.innerHTML).not.toContain(secret);
  expect(text()).not.toContain('Show credentials once');
});
