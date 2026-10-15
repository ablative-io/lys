#[derive(Clone, Default)]
struct Observations {
    events: Arc<Mutex<Vec<BTreeMap<String, String>>>>,
    spans: Arc<AtomicU64>,
    callbacks: Arc<AtomicU64>,
    locks: Arc<AtomicU64>,
    fields: Arc<AtomicU64>,
    bytes: Arc<AtomicU64>,
    nanos: Arc<AtomicU64>,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ObservationCosts {
    callbacks: u64,
    locks: u64,
    fields: u64,
    bytes: u64,
    nanos: u64,
}

fn observation_count(value: usize) -> u64 {
    match u64::try_from(value) {
        Ok(value) => value,
        Err(error) => panic!("observation_fixture_count_overflowed: {error}"),
    }
}

struct Fields(BTreeMap<String, String>);

impl Visit for Fields {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.0.insert(field.name().to_owned(), format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.0.insert(field.name().to_owned(), value.to_owned());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.0.insert(field.name().to_owned(), value.to_string());
    }
}

impl Subscriber for Observations {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        *metadata.level() <= tracing::Level::WARN
    }

    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        let mut fields = Fields(BTreeMap::new());
        attributes.record(&mut fields);
        Id::from_u64(self.spans.fetch_add(1, Ordering::Relaxed) + 1)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        let mut fields = Fields(BTreeMap::new());
        values.record(&mut fields);
        assert_ne!(span.into_u64(), 0);
    }

    fn record_follows_from(&self, span: &Id, follows: &Id) {
        assert_ne!(span.into_u64(), 0);
        assert_ne!(follows.into_u64(), 0);
    }

    fn event(&self, event: &Event<'_>) {
        let started = Instant::now();
        self.callbacks.fetch_add(1, Ordering::Relaxed);
        let mut fields = Fields(BTreeMap::new());
        event.record(&mut fields);
        self.fields
            .fetch_add(observation_count(fields.0.len()), Ordering::Relaxed);
        for (name, value) in &fields.0 {
            self.bytes
                .fetch_add(observation_count(name.len()), Ordering::Relaxed);
            self.bytes
                .fetch_add(observation_count(value.len()), Ordering::Relaxed);
        }
        self.locks.fetch_add(1, Ordering::Relaxed);
        match self.events.lock() {
            Ok(mut events) => events.push(fields.0),
            Err(error) => panic!("observation_fixture_lock_poisoned: {error}"),
        }
        let nanos = match u64::try_from(started.elapsed().as_nanos()) {
            Ok(nanos) => nanos,
            Err(error) => panic!("observation_fixture_duration_overflowed: {error}"),
        };
        self.nanos.fetch_add(nanos, Ordering::Relaxed);
    }

    fn enter(&self, span: &Id) {
        assert_ne!(span.into_u64(), 0);
    }

    fn exit(&self, span: &Id) {
        assert_ne!(span.into_u64(), 0);
    }
}

impl Observations {
    fn costs(&self) -> ObservationCosts {
        ObservationCosts {
            callbacks: self.callbacks.load(Ordering::Relaxed),
            locks: self.locks.load(Ordering::Relaxed),
            fields: self.fields.load(Ordering::Relaxed),
            bytes: self.bytes.load(Ordering::Relaxed),
            nanos: self.nanos.load(Ordering::Relaxed),
        }
    }

    fn read(&self) -> Result<Vec<BTreeMap<String, String>>, Box<dyn Error>> {
        match self.events.lock() {
            Ok(events) => Ok(events.clone()),
            Err(error) => Err(format!("observation_fixture_lock_poisoned: {error}").into()),
        }
    }
}

fn observed(events: &[BTreeMap<String, String>], step: &str, cause: &str, revision: u64) -> bool {
    events.iter().any(|fields| {
        fields.get("step").is_some_and(|value| value == step)
            && fields
                .values()
                .any(|value| value.contains(cause) && value.contains("PermissionEngineUnavailable"))
            && fields
                .get("revision")
                .is_some_and(|value| value == &revision.to_string())
    })
}
