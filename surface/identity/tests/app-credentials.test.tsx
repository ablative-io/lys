// Approval's custody on the Apps screen (DIRECTORY-081): Lys secrets makes and keeps the app's secrets before the
// approval takes effect, the screen names where they are kept and never shows either, and no route takes them back.
import { expect, it } from 'vitest';
import { click, mount, text } from './harness';
import { SERVICE, ok, refused } from './fixtures';

const BACK='https://notes.example.test/signed-in';
// The registration asks for one return address; approval keeps it as the sign-in settings, the name withheld.
const pending={id:'fixture_notes',name:'Notes fixture',state:'pending',redirects:[BACK],sign_in:null,schema:{kinds:{}},version:0,versions:[],pending:null,client_id:null,service_account:null,client_credentials:[],registered_by:{kind:'start'},registered_at:1};
const approvedApp={...pending,state:'approved',client_id:'fixture_notes',sign_in:{redirects:[BACK],profile:false,operation:'op-approve',by:{kind:'start'},at:2}};
const custody={app:'fixture_notes',client_secret_ref:'sealed-client',api_credential_ref:'sealed-api'};
const button=(label:string)=>[...document.querySelectorAll('button')].find((b)=>(b.getAttribute('aria-label')??b.textContent)===label)??null;
function routes(){let approved=false;return {...SERVICE,'/apps':()=>ok({apps:[approved?approvedApp:pending]}),
  'POST /apps/fixture_notes/approve':()=>{approved=true;return ok({app:approvedApp,credentials:custody});}};}

it('approval names where Lys secrets keeps the app’s secrets, in one request, showing neither',async()=>{
  const {posted}=await mount('#/apps',routes());
  await click(button('Approve Notes fixture'));
  expect(posted).toEqual([{path:'/apps/fixture_notes/approve',body:{operation:expect.stringMatching(/^op-/),redirects:[BACK],profile:false}}]);
  expect(text()).toContain('Lys secrets holds this app’s sign-in secret');
  expect(text()).toContain('sealed-client');expect(text()).toContain('sealed-api');
  expect(button('Save credentials in Lys secrets')).toBeNull();
});
it('an approval answer naming another app’s custody is not taken as confirmed',async()=>{
  await mount('#/apps',{...routes(),'POST /apps/fixture_notes/approve':ok({app:approvedApp,credentials:{...custody,app:'another_app'}})});
  await click(button('Approve Notes fixture'));
  expect(text()).toContain('did not confirm this app’s approval');
  expect(text()).not.toContain('Lys secrets holds this app’s sign-in secret');
});
it('refused custody leaves approval available and never retries automatically',async()=>{
  const {posted}=await mount('#/apps',{...routes(),
    'POST /apps/fixture_notes/approve':refused(502,'SecretsUnavailable','custody not confirmed')});
  await click(button('Approve Notes fixture'));
  expect(posted).toHaveLength(1);
  expect(text()).toContain('SecretsUnavailable');
  expect(text()).not.toContain('Lys secrets holds');
  expect(button('Approve Notes fixture')).not.toBeNull();
});
