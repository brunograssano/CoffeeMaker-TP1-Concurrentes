# TP1 - CoffeeGPT

El presente trabajo practico tiene como objetivo implementar una aplicacion en Rust que modele el control y reporte de una cafetera inteligente. Para esto sera necesario utilizar y aprender las herramientas de concurrencia vistas hasta el momento.

## Integrante

| Nombre                                                        | Padrón |
| ------------------------------------------------------------- | ------ |
| [Grassano, Bruno](https://github.com/brunograssano)           | 103855 |

## Ejecución

La aplicacion puede ser ejecutada a traves de `cargo` con:

```
$ cargo run
```

Adicionalmente se agregan las siguientes opciones en la ejecucion:
* Se puede indicar un archivo de ordenes distinto al por defecto (`orders.json`)
* Se puede cambiar el nivel de log con la variable de entorno `RUST_LOG`. Algunos valores posibles son `error`, `info`, y `debug`

De forma completa quedaria:
```
$ RUST_LOG=info cargo run my-orders.json
```

### Tests

Se proveen distintos casos de prueba de la aplicacion. Se pueden ejecutar con:
```
$ cargo test
```

Algunas pruebas destacadas son:
* Se prueba con un archivo que no existe
* Se prueba con un archivo vacio
* Se prueba con un formato equivocado
* Se prueba con ordenes en cantidad
* Se prueba que recargue los contenedores
* Se prueba que saltee ordenes en caso de agotarse los recursos

*Nota: Algunas pruebas se hacen considerando que los valores iniciales de los recursos son de 5000.*

## Diseño e implementación

### Formato del archivo
Para la lectura de ordenes de un archivo se utiliza el crate `serde` para leer y procesar archivos con formato JSON.

Este archivo tiene que seguir el siguiente formato:
```json
{
    "orders": [
        {
            "ground_coffee": 100,
            "hot_water": 150,
            "cacao": 60,
            "milk_foam": 70
        }
        // más ordenes...
    ]
}
```

Las ordenes pueden estar conformadas por cafe (`ground_coffee`), agua caliente (`hot_water`), cacao (`cacao`) y espuma de leche (`milk_foam`). Cada una de estas cantidades tiene que ser un entero positivo o cero.

En caso de no respetarse el formato (por ejemplo, numeros negativos o tipos erroneos) se imprimira por pantalla un mensaje de error y finalizara la ejecucion.

### Threads y comunicación

El modelo de la aplicacion se puede representar a traves de los siguientes diagramas.

![Relaciones entre las estructuras de la aplicación](docs/relationships.jpg)

TODO tildes

En este diagrama podemos ver la estructura en forma de objetos, como son las relaciones entre las distintas entidades. Tenemos las siguientes:
* `Order` representa a un pedido de la cafetera. Esta compuesto por los ingredientes y cantidades que necesita. 
    * Se tomo el supuesto de que un pedido no puede necesitar mas recurso que lo definido en `MAX_OF_INGREDIENT_IN_AN_ORDER`. Al no alcanzar el recurso almacenado para cubrir una orden con el maximo establecido se recargara el contenedor si corresponde. Se toma este supuesto para simplificar el proceso de despertar los reponedores de recursos en vez de estar llevando a cero el recurso del contenedor y luego reponer. 
    * Se realizo una optimizacion en las ordenes al hacer que los ingredientes sean recibidos en un vector que no sigue un orden en particular. De esta forma se busca mejorar la performance al momento de armar la orden en el dispenser. Esto se puede ver en `get_ingredients_from_order(...)` de `orders_reader.rs`.
* `OrderReader` es el encargado de realizar la lectura de las ordenes del archivo JSON. Este lee el archivo, realiza el parseo, y luego comienza a enviar los pedidos a traves de `OrdersQueue`. Por cada orden despierta a los dispensers en caso de que esten esperando para realizar una orden. Al ir cargando de a uno este pedido se va simulando el arribo de los clientes con los pedidos. *Nota: No esta implementado con un struct, es una funcion que cumple el rol.*
* `Container`, representa a un contenedor de la cafetera. Lleva el registro de cuanto queda de recurso y cuanto se fue consumiendo.
* `Resources` viene a agrupar a los distintos recursos que tiene la cafetera. Esta implementado con un mapa donde la clave es el nombre del recurso y el valor el contenedor. Se opto por esta estructura de datos para reducir la cantidad de `ifs` que habria al ir procesando las ordenes en un dispenser.
* `Dispenser` es un dispensador de la cafetera. Estos obtienen las ordenes de la `OrdersQueue` y las procesan en el orden que venga el vector de ingredientes (en este punto se ven las optimizaciones mencionadas previamente).
    * En caso de que no alcance el recurso actual para cumplir lo requerido, despertara a los reponedores que se encargaran del proceso. Se opto por despertar a todos los reponedores para no estar complicando el codigo con chequeos y variables condicionales adicionales.
    * Si pasado el proceso de despertar a los reponedores sigue sin alcanzar el recurso (porque se acabo o no quedaba suficiente), se descarta la orden y se pierden los recursos utilizados hasta el momento. Se considera como si ya se hubieran tirado al vaso de la cafetera.
* `StatisticsPrinter`, es la estructura que va imprimiendo las estadisticas de uso y alarmas de bajo nivel de recurso.
    * El tiempo de espera se define en la constante `STATISTICS_WAIT_IN_MS`. Notar que la impresion de la estadistica puede llevar mas tiempo, ya que se esta intentando acceder a distintos locks que pueden estar en uso por las otras entidades.
    * El nivel de alerta esta definido en `X_PERCENTAGE_OF_CAPACITY`. Cuando los contenedores de granos, leche y cacao se encuentran por debajo de ese porcentaje de capacidad se imprime por pantalla un mensaje de aviso del contenedor. El valor tiene que estar entre 0 y 100. 
* `ExternalReplenisher` y `ContainerReplenisher` son los reponedores de recursos. Se despiertan cuando el nivel del recurso que manejan es inferior a `MAX_OF_INGREDIENT_IN_AN_ORDER`. Al hacerlo toman el control de los conenedores que manejan y los recargan.
    *  `ExternalReplenisher` simula la recarga del mismo contenedor desde una fuente externa. En este caso es solamente el contenedor de agua que estaria tomando el agua de la red.
    * `ContainerReplenisher` simula el proceso de tomar recursos de un contenedor, convertirlos y cargar el contenedor deseado. Serian los recursos de cafe y leche.
    * El tiempo de espera que se tiene es `MINIMUM_WAIT_TIME_REPLENISHER` mas la cantidad que se esta reponiendo de recurso.
* `CoffeeMaker` inicia la cafetera, indica a los threads que deben de terminar, y los espera. Es el punto de entrada al sistema.  


![Threads de la aplicación](docs/threads.jpg)

TODO

### Dificultades encontradas

### Documentación
La documentacion de la aplicacion se puede ver con:
```
$ cargo doc --open
```
