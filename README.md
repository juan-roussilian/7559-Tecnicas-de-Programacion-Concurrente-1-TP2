# TP2 - CoffeeGPT

El presente trabajo práctico tiene como objetivo implementar aplicaciones en Rust que modelen un sistema de puntos para fidelizacion de los clientes. Los clientes podran sumar puntos por cada compra para canjearlos por cafes gratuitos.

Estas aplicaciones deben de trabajar en ambientes distribuidos susceptibles a fallas debido a perdida de conexion.

## Integrantes

| Nombre                                                        | Padrón |
| ------------------------------------------------------------- | ------ |
| [Grassano, Bruno](https://github.com/brunograssano)           | 103855 |
| [Roussilian, Juan Cruz](https://github.com/juan-roussilian)   | 104269 |
| [Stancanelli, Guillermo](https://github.com/guillermo-st)     | 104244 |

## Ejecucion

La aplicación puede ser ejecutada a través de `cargo` con:

```
$ cargo run --bin [NOMBRE-APP] [ARGUMENTOS]
```

* Donde `[NOMBRE-APP]` puede ser `server` o `coffee_maker`
* Los valores de `[ARGUMENTOS]` dependen de la aplicacion que se quiere ejecutar.
    * En el caso del server son `[ID] [TOTAL-SERVIDORES]` donde `[ID]` es el id del servidor (se debe de empezar con 0) y `[TOTAL-SERVIDORES]` la cantidad total de servidores que puede tener la red. Siempre se debe de iniciar el servidor 0 para que comience a funcionar correctamente.
    * En el caso de la cafetera `[IP:PORT] [FILE]` donde `[IP:PORT]` tiene la ip y puerto del servidor al que se va a conectar la cafetera y `[FILE]` el nombre del archivo. El nombre del archivo es opcional, si no se incluye se lee el ubicado en `tests/orders.csv`
* Se puede cambiar el nivel de log con la variable de entorno `RUST_LOG`. Algunos valores posibles son `error`, `info`, y `debug`

De forma completa quedaría:
```
$ RUST_LOG=info cargo run --bin server 0 5
$ RUST_LOG=info cargo run --bin coffee_maker 127.0.0.1:10000 tests/orders.csv
```

### Tests

Se proveen distintos casos de prueba de la aplicación. Se pueden ejecutar con:
```
$ cargo test
```

Algunas pruebas destacadas son:

### Dependencias
La aplicación tiene las siguientes dependencias:

## Diseño e implementación

### Formato del archivo

### Modelo

## Dificultades encontradas

## Documentación
La documentación de la aplicación se puede ver con:
```
$ cargo doc --open
```