# TP2 - CoffeeGPT

El presente trabajo práctico tiene como objetivo implementar aplicaciones en Rust que modelen un sistema de puntos para fidelización de los clientes. Los clientes podrán sumar puntos por cada compra para canjearlos por cafés gratuitos.

Estas aplicaciones deben de trabajar en ambientes distribuidos susceptibles a fallas debido a perdida de conexión.

## Integrantes

| Nombre                                                        | Padrón |
| ------------------------------------------------------------- | ------ |
| [Grassano, Bruno](https://github.com/brunograssano)           | 103855 |
| [Roussilian, Juan Cruz](https://github.com/juan-roussilian)   | 104269 |
| [Stancanelli, Guillermo](https://github.com/guillermo-st)     | 104244 |

## Ejecución

La aplicación puede ser ejecutada a través de `cargo` con:

```
$ cargo run --bin [NOMBRE-APP] [ARGUMENTOS]
```

* Donde `[NOMBRE-APP]` puede ser `server` o `coffee_maker`
* Los valores de `[ARGUMENTOS]` dependen de la aplicación que se quiere ejecutar.
    * En el caso del server son `[ID] [TOTAL-SERVIDORES]` donde `[ID]` es el id del servidor (se debe de empezar con 0) y `[TOTAL-SERVIDORES]` la cantidad total de servidores que puede tener la red. Siempre se debe de iniciar el servidor 0 para que comience a funcionar correctamente.
    * En el caso de la cafetera `[IP:PORT] [FILE]` donde `[IP:PORT]` tiene la ip y puerto del servidor al que se va a conectar la cafetera y `[FILE]` el nombre del archivo. El nombre del archivo es opcional, si no se incluye se lee el ubicado en `tests/orders.csv` (definido por la constante `DEFAULT_ORDERS_FILE`)
* Se puede cambiar el nivel de log con la variable de entorno `RUST_LOG`. Algunos valores posibles son `error`, `info`, y `debug`

De forma completa quedaría:
```
$ RUST_LOG=info cargo run --bin server 0 5
$ RUST_LOG=info cargo run --bin coffee_maker 127.0.0.1:20000 tests/orders.csv
```

### Tests

Se proveen distintos casos de prueba de la aplicación. Se pueden ejecutar con:
```
$ cargo test
```

Algunas pruebas destacadas son:

### Dependencias y binarios
El trabajo práctico está dividido en las siguientes partes:
* Un binario para las cafeteras, `coffee_maker`
* Un binario para los servidores, `server`
* Una biblioteca con funcionalidades comunes a ambos binarios, `lib`


La aplicación tiene las siguientes dependencias:

* `rand` para generar números pseudoaleatorios, es usado para determinar el éxito de los pedidos.
* `actix` y `actix-rt` para el manejo de actores.
* `log` y `simple_logger` para tener la interfaz de los logs *(error!(), info!(), debug!())* y una implementación que imprime los mensajes.
* `async-std` para el manejo de tareas asincrónicas
* `async-trait` para poder definir interfaces con métodos *async*
* `bincode` y `serde` para serializar y deserializar a bytes los mensajes enviados.


## Diseño e implementación

### Arquitectura

La arquitectura del trabajo es de la siguiente forma:
![Arquitectura del trabajo](docs/arquitectura.png)

* Se tienen múltiples servidores locales que replican la base de datos de los puntos y están conectados entre sí
* Cada servidor local puede manejar múltiples conexiones de cafeteras

### Cafetera

Empezamos por la cafetera, la aplicación de la cafetera simula ser la máquina que hace el café en cada pedido. Estos pedidos son leídos de un archivo.

#### Formato del archivo

La cafetera para procesar los pedidos debe de leerlos de un archivo CSV que sigue el siguiente formato `OPERACION,COSTO/BENEFICIO,NRO CUENTA`. Donde:
* `OPERACION` es el tipo de pedido, puede ser de `CASH` para sumar puntos o `POINTS` para restar puntos. 
* `COSTO/BENEFICIO` es la cantidad que se va a sumar o restar de puntos. Es un número positivo
* `NRO CUENTA` es el id numérico positivo de la cuenta que realiza la operación.

Por ejemplo:


```
CASH,200,4
POINTS,200,2
POINTS,200,11
CASH,200,12
...
```

En caso de no respetarse el formato en una línea, se salteara e intentara leer la siguiente, siempre y cuando el archivo tenga un formato válido de UTF-8. Por ejemplo

```
CASH,200,4,442 <--- Falla la lectura y reintenta
POINTasdS,200,2 <--- Falla la lectura y reintenta
POINTS,200,-11 <--- Falla la lectura y reintenta
CASH,200,12 <--- Lee y parsea correctamente
...
```

#### Modelo

El modelo de la cafetera es el siguiente:
![Modelo de la cafetera](docs/modelo-cafetera.png)

En el diagrama podemos ver que la cafetera se puede dividir en dos partes que se comunican mediante mensajes, el lector de ordenes `OrdersReader` y la lógica del negocio en `CoffeeMaker`. Estas dos entidades están modeladas como actores.
* `OrdersReader` realiza la lectura y parseo del archivo CSV línea por línea a pedido de las cafeteras. Una vez realizada la lectura le responde a la cafetera con el pedido que tiene que realizar. Si ocurre un error en la lectura se envía un mensaje a sí mismo para que reintente y lea otra línea para esa misma cafetera.
* `CoffeeMaker` es el otro actor del modelo. Este actor realiza los pedidos de suma y resta. Cada uno tarda el tiempo definido en la constante `PROCESS_ORDER_TIME_IN_MS`.
    * Para saber si los pedidos fueron exitosos o no se separó la funcionalidad con el trait `Randomizer`. La probabilidad de éxito se define en la constante `SUCCESS_CHANCE`. Este trait adicionalmente permite manejar la parte pseudoaleatoria en los tests al usar mocks.
    * Para la comunicación con el servidor local se creó el cliente `LocalServerClient`. Este cliente se encarga de realizar y mantener la conexión.
    * `Protocol` es una interfaz para no acoplar la conexión a un protocolo de transporte en particular. La cafetera se conecta mediante TCP con el servidor local.
    * Si bien en el diagrama aparece como que hay una sola cafetera, puede configurarse mediante la constante `DISPENSERS` para que haya múltiples actores de este tipo. *Esto es para reducir la cantidad de aplicaciones a levantar.*

#### Actores y mensajes

En el siguiente diagrama se puede ver la comunicación entre los actores mencionados previamente.

![Comunicación entre los actores](docs/mensajes-cafetera.png)

1. El ciclo empieza una vez que `main` envía el mensaje `OpenFile` con las direcciones de las cafeteras a `OrdersReader`. El lector se va a guardar las direcciones y abrir el archivo.
2. Si se logra abrir exitosamente se les notifica a los actores de `CoffeeMaker` que se abrió con `OpenedFile`
3. Las cafeteras responden con el mensaje de `ReadAnOrder` para que el lector lea.
4. El lector le responde a cada cafetera que pedido tiene que atender en `ProcessOrder`
5. La cafetera procesa el pedido y vuelve a pedir otra orden.
6. Se repiten los pasos 4 y 5 hasta que se termine el archivo.

#### Comunicación con Servidor Local

Como ya se mencionó antes, para la comunicación cafetera-servidor local optamos por usar el protocolo de transporte TCP. Optamos por este protocolo debido a que garantiza que los datos serán entregados al servidor sin errores, en orden, y que la conexión con el servidor está activa.
La alternativa, UDP no garantiza nada de lo anterior, por lo que implicaba un desarrollo adicional para asegurar las propiedades mencionadas, principalmente los ACK y orden. 

Sin embargo, en la implementación se deja la libertad de intercambiar el protocolo empleado, ya que se tiene la interfaz `ConnectionProtocol`.

Pasando a los mensajes usados, se buscó tener un formato bien definido que sea independiente del tipo de pedido. Para eso definimos los campos comunes y se llegó a lo siguiente:

```rust
pub struct CoffeeMakerRequest {

    pub message_type: MessageType,
    pub account_id: usize,
    pub points: usize,
}

pub struct CoffeeMakerResponse {

    pub message_type: MessageType,
    pub status: ResponseStatus,
}
```

* `MessageType` y `ResponseStatus` son *enums* que tienen las distintas acciones/resultados.
* Los *structs* son serializados y deserializados mediante el crate `bincode` y `serde`.
* A los bytes enviados se le agrega al final el byte `;` para leer hasta ese punto.


### Servidor local

## Dificultades encontradas

## Documentación
La documentación de la aplicación se puede ver con:
```
$ cargo doc --open
```