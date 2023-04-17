//! Parametros de configuracion de la cafetera

/// Cantidad de dispensadores que tiene la cafetera
pub const N_DISPENSERS: usize = 10;

/// Capacidad inicial de granos para moler
pub const G_STORAGE: u64 = 5000;

/// Capacidad inicial de granos molidos (cafe)
pub const M_STORAGE: u64 = 5000;

/// Capacidad inicial de leche fria
pub const L_STORAGE: u64 = 5000;

/// Capacidad inicial de espuma de leche
pub const E_STORAGE: u64 = 5000;

/// Capacidad inicial de cacao
pub const C_STORAGE: u64 = 5000;

/// Capacidad inicial de agua caliente
pub const A_STORAGE: u64 = 5000;

/// Indica cuanto tiempo se debe de esperar (por lo menos) para imprimir por pantalla las estadisticas de la cafetera
pub const STATISTICS_WAIT_IN_MS: u64 = 50;

/// Porcentaje a partir del cual se va a alertar de que se acaba un contenedor
pub const X_PERCENTAGE_OF_CAPACITY: u64 = 20;

/// Cantidad maxima que puede tener un ingrediente en una orden. Suponiendo que es 2500, no puede haber más de 2500 de cafe en la orden.
pub const MAX_OF_INGREDIENT_IN_AN_ORDER: u64 = 2500;

/// Tiempo minimo de espera de los reponedores de ingredientes
pub const MINIMUM_WAIT_TIME_REPLENISHER: u64 = 100;
