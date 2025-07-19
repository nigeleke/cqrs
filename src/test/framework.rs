use crate::aggregate::Aggregate;
use crate::mem_store::MemStore;
use crate::mem_store::MemStoreAggregateContext;
use crate::query::Query;
use crate::store::{AggregateContext, EventStore};
use crate::test::AggregateTestExecutor;
use crate::Reactor;

/// A framework for rigorously testing the aggregate logic, one of the *most important*
/// parts of any DDD system. Note: the defaulted [AggregateContext] and [Store] types
/// are incidental as the default `context_store` is None. The types are defined when
/// using [`using_context_and_store`] and [`using_mem_store`].
pub type TestFramework<A> = GenericTestFramework<A, MemStoreAggregateContext<A>, MemStore<A>>;

/// The framework implementation.
pub struct GenericTestFramework<A, AC, S>
where
    A: Aggregate,
    AC: AggregateContext<A> + Send + Sync,
    S: EventStore<A, AC = AC>,
{
    service: A::Services,
    queries: Vec<Box<dyn Query<A>>>,
    reactors: Vec<Box<dyn Reactor<A, AC>>>,
    context_store: Option<(AC, S)>,
}

impl<A: Aggregate> GenericTestFramework<A, MemStoreAggregateContext<A>, MemStore<A>> {
    /// Create a test framework using the provided service. No event store, queries or reactors are defined.
    pub fn with(service: A::Services) -> Self {
        let queries = Vec::default();
        let reactors = Vec::default();
        let context_store = None;
        Self {
            service,
            queries,
            reactors,
            context_store,
        }
    }

    /// Use an event store within the current test framework.
    pub fn using_context_and_store<SO, ACO>(
        self,
        context: ACO,
        store: SO,
    ) -> GenericTestFramework<A, ACO, SO>
    where
        ACO: AggregateContext<A> + Send + Sync,
        SO: EventStore<A, AC = ACO>,
    {
        let service = self.service;
        let queries = self.queries;
        if !self.reactors.is_empty() {
            panic!("reactors must be added after context and store defined")
        }
        let reactors = Vec::default();
        let context_store = Some((context, store));
        GenericTestFramework {
            service,
            queries,
            reactors,
            context_store,
        }
    }

    /// Use a [MemStore] event store within the current test framework.
    pub fn using_mem_store(
        self,
    ) -> GenericTestFramework<A, MemStoreAggregateContext<A>, MemStore<A>> {
        self.using_context_and_store(MemStoreAggregateContext::default(), MemStore::default())
    }
}

impl<A, AC, S> GenericTestFramework<A, AC, S>
where
    A: Aggregate,
    AC: AggregateContext<A> + Send + Sync,
    S: EventStore<A, AC = AC>,
{
    /// Add a query into the current test framework. An event store must be defined
    /// before providing pre-conditions with ([given_no_previous_events] / [given]).
    pub fn and_query(self, query: Box<dyn Query<A>>) -> Self {
        let service = self.service;
        let mut queries = self.queries;
        queries.push(query);
        let reactors = self.reactors;
        let context_store = self.context_store;
        Self {
            service,
            queries,
            reactors,
            context_store,
        }
    }

    /// Add all queries into the current test framework. An event store must be defined
    /// before providing pre-conditions ([given_no_previous_events] / [given]).
    pub fn and_queries(self, queries: Vec<Box<dyn Query<A>>>) -> Self {
        queries.into_iter().fold(self, |acc, q| acc.and_query(q))
    }

    /// Add a reactor into the current test framework. An event store must be defined
    /// before providing reactors because reactors need the concrete [AggregateContext]
    /// type.
    pub fn and_reactor(self, reactor: Box<dyn Reactor<A, AC>>) -> Self {
        let service = self.service;
        let queries = self.queries;
        let mut reactors = self.reactors;
        reactors.push(reactor);
        let context_store = self.context_store;
        Self {
            service,
            queries,
            reactors,
            context_store,
        }
    }

    /// Add all reactors into the current test framework. An event store must be defined
    /// before providing reactors because reactors need the concrete [AggregateContext]
    /// type.
    pub fn and_reactors(self, reactors: Vec<Box<dyn Reactor<A, AC>>>) -> Self {
        reactors.into_iter().fold(self, |acc, r| acc.and_reactor(r))
    }

    /// Initiates an aggregate test with no previous events.
    ///
    /// ```
    /// # use cqrs_es::doc::{MyAggregate, MyService};
    /// use cqrs_es::test::TestFramework;
    ///
    /// let executor = TestFramework::<MyAggregate>::with(MyService)
    ///     .given_no_previous_events();
    /// ```
    #[must_use]
    pub fn given_no_previous_events(self) -> AggregateTestExecutor<A, AC, S> {
        AggregateTestExecutor::new(Vec::new(), self.service, self.queries, self.context_store)
    }
    /// Initiates an aggregate test with a collection of previous events.
    ///
    /// ```
    /// # use cqrs_es::doc::{MyAggregate, MyEvents, MyService};
    /// use cqrs_es::test::TestFramework;
    ///
    /// let executor = TestFramework::<MyAggregate>::with(MyService)
    ///     .given(vec![MyEvents::SomethingWasDone]);
    /// ```
    #[must_use]
    pub fn given(self, events: Vec<A::Event>) -> AggregateTestExecutor<A, AC, S> {
        AggregateTestExecutor::new(events, self.service, self.queries, self.context_store)
    }
}
