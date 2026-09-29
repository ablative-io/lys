import { expect, it } from 'vitest';
import { click, mount, text } from './harness';
import { SERVICE, ok, refused } from './fixtures';

const secret='f'.repeat(64);
const pending={id:'fixture_notes',name:'Notes fixture',state:'pending',redirects:[],schema:{kinds:{}},version:0,versions:[],pending:null,client_id:null,service_account:null,registered_by:{kind:'start'},registered_at:1};
const button=(label:string)=>[...document.querySelectorAll('button')].find((b)=>b.textContent===label)??null;
function routes(){let approved=false;return {...SERVICE,'/apps':()=>ok({apps:[approved?{...pending,state:'approved'}:pending]}),
  'POST /apps/fixture_notes/approve':()=>{approved=true;return ok({app:{...pending,state:'approved'},client:{client_id:'fixture_notes',client_secret:secret,credential:'lys-app.fixture_notes.'+secret}});}};}
it('saves without copying and removes credential values after broker confirmation',async()=>{
  const {posted}=await mount('#/apps',{...routes(),'POST /apps/fixture_notes/credentials/save':ok({app:'fixture_notes',client_secret_ref:'sealed-client',api_credential_ref:'sealed-api'})});
  await click(button('Approve Notes fixture'));await click(button('Save credentials in Lys secrets'));
  expect(posted[1]).toEqual({path:'/apps/fixture_notes/credentials/save',body:{client_secret:secret}});
  expect(text()).toContain('Credentials saved in Lys secrets');expect(text()).not.toContain(secret);
  expect(JSON.stringify(Object.entries(sessionStorage))).not.toContain(secret);expect(JSON.stringify(Object.entries(localStorage))).not.toContain(secret);
});
it('a refused save keeps the issued credential only in memory and makes no automatic retry',async()=>{
  const {posted}=await mount('#/apps',{...routes(),'POST /apps/fixture_notes/credentials/save':refused(503,'SecretsUnavailable','unknown outcome')});
  await click(button('Approve Notes fixture'));await click(button('Save credentials in Lys secrets'));
  expect(posted).toHaveLength(2);expect(text()).toContain('SecretsUnavailable');expect(text()).toContain('No automatic retry');expect(text()).not.toContain('Credentials saved');
  await click(button('Save credentials in Lys secrets'));expect(posted).toHaveLength(3);expect(posted[2]).toEqual(posted[1]);
});
it('a malformed success never clears the only issued copy or claims saved',async()=>{
  await mount('#/apps',{...routes(),'POST /apps/fixture_notes/credentials/save':ok({app:'another-app',client_secret_ref:'x',api_credential_ref:'y'})});
  await click(button('Approve Notes fixture'));await click(button('Save credentials in Lys secrets'));
  expect(text()).toContain('did not confirm');expect(text()).not.toContain('Credentials saved');expect(button('Save credentials in Lys secrets')).not.toBeNull();
});
