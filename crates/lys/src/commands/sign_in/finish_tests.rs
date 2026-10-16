use super::{finish, validate};
use std::error::Error;
use std::net::TcpListener;

#[test]
fn finish_refuses_unsafe_addresses_before_connecting() {
    for address in ["https://127.0.0.1:1234/lys/sign-in?state=x", "http://example.test:1234/lys/sign-in?x", "http://127.0.0.1/lys/sign-in?x", "http://127.0.0.1:1234/other?x", "http://127.0.0.1:1234/lys/sign-in", "http://user@127.0.0.1:1234/lys/sign-in?x", "http://127.0.0.1:1234/lys/sign-in?x#fragment"] {
        assert!(validate(address).err().is_some_and(|e|e.to_string().contains("sign_in_finish_invalid")));
    }
    assert!(validate("http://[::1]:1234/lys/sign-in?state=x").is_ok());
}

#[test]
fn finish_uses_the_callback_receiver_for_success_and_wrong_state() -> Result<(), Box<dyn Error>> {
    for state in ["expected","wrong"] {
        let listener=TcpListener::bind("127.0.0.1:0")?;
        let address=format!("http://{}/lys/sign-in?state={state}&code=fixture",listener.local_addr()?);
        let receiver=std::thread::spawn(move || super::super::callback::callback(&listener,"expected").map(|s|s.to_string()).map_err(|e|e.to_string()));
        let answer=finish(&address);
        let received=receiver.join().map_err(|_|"receiver panicked")?;
        if state=="expected" { answer?; assert_eq!(received?,"fixture"); }
        else { assert!(answer.err().ok_or("wrong state accepted")?.to_string().contains("sign_in_state_wrong")); assert!(received.is_err()); }
    }
    Ok(())
}
