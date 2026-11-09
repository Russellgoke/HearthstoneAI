//! hsengine: flat, Copy-able Hearthstone-style engine for ISMCTS / RL.
//!
//! Python bindings are compiled only with the `python` feature (maturin sets it).

pub mod action;
pub mod cards;
pub mod engine;
pub mod hash;
pub mod rollout;
pub mod serializer;
pub mod state;

pub use action::{Action, ActionList, MAX_ACTIONS};
pub use rollout::random_rollout;
pub use state::{CardId, GameState, GameStatus, Minion, PlayerState};

#[cfg(feature = "python")]
mod python {
    use pyo3::exceptions::PyValueError;
    use pyo3::prelude::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    use crate::action::{Action, MAX_ACTIONS};
    use crate::engine::DEFAULT_DECK_SIZE;
    use crate::state::{GameState, MAX_DECK};
    use crate::{rollout, serializer};

    fn rng_from(seed: Option<u64>) -> SmallRng {
        match seed {
            Some(s) => SmallRng::seed_from_u64(s),
            None => SmallRng::from_os_rng(),
        }
    }

    #[pyclass(name = "GameState", module = "hsengine")]
    #[derive(Clone)]
    pub struct PyGameState {
        inner: GameState,
    }

    #[pymethods]
    impl PyGameState {
        /// New game with two random decks.
        #[new]
        #[pyo3(signature = (seed=None, deck_size=DEFAULT_DECK_SIZE))]
        fn new(seed: Option<u64>, deck_size: usize) -> PyResult<Self> {
            if deck_size > MAX_DECK {
                return Err(PyValueError::new_err(format!("deck_size must be <= {MAX_DECK}")));
            }
            let mut rng = rng_from(seed);
            Ok(Self { inner: GameState::new_random_game(&mut rng, deck_size) })
        }

        #[staticmethod]
        fn from_json(json: &str) -> PyResult<Self> {
            serializer::from_json(json)
                .map(|inner| Self { inner })
                .map_err(|e| PyValueError::new_err(e.to_string()))
        }

        #[pyo3(signature = (pretty=true))]
        fn to_json(&self, pretty: bool) -> String {
            if pretty {
                serializer::to_json(&self.inner)
            } else {
                serializer::to_json_compact(&self.inner)
            }
        }

        /// Legal action ids (integers in 0..MAX_ACTIONS), ascending.
        fn get_legal_actions(&self) -> Vec<usize> {
            let mut v = Vec::with_capacity(32);
            self.inner.for_each_legal(|a| v.push(a.to_index()));
            v
        }

        /// Boolean mask of length MAX_ACTIONS.
        fn get_action_mask(&self) -> Vec<bool> {
            self.inner.get_action_mask().to_vec()
        }

        /// Apply one atomic action by id. Raises ValueError if illegal.
        fn step(&mut self, action_id: usize) -> PyResult<()> {
            let a = Action::from_index(action_id).ok_or_else(|| {
                PyValueError::new_err(format!("action id {action_id} out of range 0..{MAX_ACTIONS}"))
            })?;
            self.inner.try_apply_action(a).map_err(PyValueError::new_err)
        }

        /// Plays random actions to the end on a copy; returns winner (0/1) or None for a draw.
        #[pyo3(signature = (seed=None))]
        fn random_rollout(&self, seed: Option<u64>) -> Option<u8> {
            let mut s = self.inner;
            let mut rng = rng_from(seed);
            rollout::random_rollout(&mut s, &mut rng).0.winner()
        }

        /// Hash of the state visible to `perspective` (defaults to the active player).
        #[pyo3(signature = (perspective=None))]
        fn observable_hash(&self, perspective: Option<u8>) -> u64 {
            self.inner.observable_hash(perspective.unwrap_or(self.inner.current))
        }

        fn copy(&self) -> Self {
            self.clone()
        }

        fn __copy__(&self) -> Self {
            self.clone()
        }

        fn __deepcopy__(&self, _memo: &Bound<'_, PyAny>) -> Self {
            self.clone()
        }

        fn __eq__(&self, other: &Self) -> bool {
            self.inner == other.inner
        }

        #[getter]
        fn current_player(&self) -> u8 {
            self.inner.current
        }

        #[getter]
        fn turn(&self) -> u16 {
            self.inner.turn
        }

        #[getter]
        fn is_terminal(&self) -> bool {
            self.inner.is_terminal()
        }

        /// 0 or 1 if someone has won, else None (ongoing or draw).
        #[getter]
        fn winner(&self) -> Option<u8> {
            self.inner.status.winner()
        }

        #[getter]
        fn status(&self) -> String {
            format!("{:?}", self.inner.status)
        }

        fn __repr__(&self) -> String {
            let p = &self.inner.players;
            format!(
                "GameState(turn={}, current={}, hp=[{}, {}], board=[{}, {}], hand=[{}, {}], status={:?})",
                self.inner.turn,
                self.inner.current,
                p[0].hero_health,
                p[1].hero_health,
                p[0].board_len,
                p[1].board_len,
                p[0].hand_len,
                p[1].hand_len,
                self.inner.status
            )
        }
    }

    /// Human-readable description of an action id.
    #[pyfunction]
    fn action_to_string(action_id: usize) -> PyResult<String> {
        Action::from_index(action_id)
            .map(|a| a.to_string())
            .ok_or_else(|| PyValueError::new_err(format!("action id {action_id} out of range")))
    }

    #[pymodule]
    fn hsengine(m: &Bound<'_, PyModule>) -> PyResult<()> {
        m.add_class::<PyGameState>()?;
        m.add_function(wrap_pyfunction!(action_to_string, m)?)?;
        m.add("MAX_ACTIONS", MAX_ACTIONS)?;
        Ok(())
    }
}
