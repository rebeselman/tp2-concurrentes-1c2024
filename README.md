# GridRust

<p align="center">
  <img src="img/grido.png" width="45%">
&nbsp; &nbsp; &nbsp; &nbsp;
  <img src="img/rust_crab.png" width="45%">
</p>

[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-24ddc0f5d75046c5622901739e7c5dd533143b0c8e959d652212380cedb1ea36.svg)](https://classroom.github.com/a/PKo6zpFB)

## Índice
- [GridRust](#gridrust)
  - [Índice](#índice)
  - [Diseño](#diseño)
    - [Interfaces de clientes](#interfaces-de-clientes)
    - [Gestión de pedidos con robots](#gestión-de-pedidos-con-robots)
    - [Gateway de pagos](#gateway-de-pagos)
  - [Supuestos](#supuestos)
  - [Conclusión](#conclusión)

## Diseño
Se tienen tres aplicaciones distintas. Una que modela las pantallas con las que eligen pedidos los clientes, gestión de pedidos (robots que realizan los pedidos) y el gateway de pagos, que es donde se captura y efectiviza el pago. Estas se comunican a través de sockets TCP.

![Diagrama del Proyecto](img/diagrams/C4_gridrust.drawio.png)

### Interfaces de clientes
- Se modela una interfaz de cliente o pantalla como una función que lee de un archivo pedidos simulados y los convierte en [ordenes de pedidos], inicia una transacción que en este caso es un [pedido de helado] (Se utilizaría programación asincrónica para esperar por la respuesta y mientras tanto dar oportunidad a atender otro pedido). Se generan varias instancias (hilos de ejecución) de esta función para simular un número constante de pantallas en la heladería.
- [Commit de dos fases]: Se tiene un [coordinador] general para todos los pedidos que se hagan por las pantallas que envía un mensaje de prepare a la aplicación de Gestión de Pedidos y a Gateway De pagos, y debería aguardar para que le confirmen de ambos lugares que se va a poder realizar el pedido. Si se confirma la posibilidad del pedido se hace el cobro efectivo y se entrega el pedido satisfactoriamente.
- Algoritmo: En este caso el compromiso es entregar el helado solicitado;
  	1.Hay un coordinador que ejecuta la orden de pedido, este escribe en su log prepare indicando que inicia la preparación del pedido y le envía a Gestión de Pedidos y Gateway de 	Pagos el mensaje prepare, para preguntar si pueden confirmar el pedido.
 	2. Cuando ambos  reciben el mensaje verifican si pueden efectuar la orden de pedido y lo escriben en el log y envían su decisión.
	3. Si el coordinador recibe todas las respuestas diciendo que están listos para comprometerse se efectúa y finaliza el compromiso, si alguno no se puede comprometer se aborta la 	preparación de la orden de pedido.


### Gestión de pedidos con robots
- **Modelo de actores** para los robots:
Tienen como estado interno el contenedor que están usando o si no están usando ninguno. Los tipos de mensajes serían para solicitar un contenedor, para liberarlo, para otorgarlo y para denegarlo. 

- **Algoritmo centralizado** para sincronizar los accesos a los contenedores de helado por parte de los robots:
Se elige a un robot como coordinador. Si un robot quiere usar alguno de los contenedores de helado le envía un mensaje de solicitud al coordinador, donde indica qué contenedor quiere usar y si ningún otro robot lo está usando el coordinador le responde _OK_ y lo deja entrar. En cambio, si ya hay algún robot utilizando ese contenedor el coordinador le envía _ACK_ y se bloquea el solicitante, agregando su solicitud a una cola. Cuando el robot que estaba usando el contenedor termina le avisa al coordinador y este saca al solicitante de la cola y para otorgarle el acceso al contenedor enviándole _OK_.
	
  Justificación: se cita el libro de Distributed Operating Systems de Tanenbaum: "El algoritmo centralizado es el más sencillo y también el más eficiente. Sólo requiere de tres mensajes para entrar y salir de una región critica: una solicitud y otorgamiento para entrar y una liberación para salir". El único problema que puede ocurrir es que falle el coordinador, pero existen algoritmos para detectar esto y elegir otro.

- **Algoritmo Bully** para elegir robot coordinador al inicio y en caso de que falle (cuando un robot observa que el coordinador ya no responde las solicitudes por un timeout que se define), inicia una elección:
  1. El robot envía _ELECTION_ a los demás procesos con un número mayor.
  2. Si nadie responde, este gana la elección y se convierte en el coordinador.
  3. Si uno de los robots con un número mayor responde, toma el control y el trabajo del robot que llamó a elecciones termina.

  Por lo visto en la bibliografía no hay mucha diferencia entre los algoritmos de elección, no hay ventajas significativas entre elegir uno u otro. Por otro lado, podría realizarse la elección con los robots comunicandose entre sí a través de sockets en vez de mensajes.
### Gateway de pagos
Sería una aplicación simple que loguea. (cito enunciado)
Esta aplicación se encargaría de recibir del coordinador que se encuentra en [Interfaces de clientes], mensajes de prepare preguntando si se puede capturar el pago, si la tarjeta no falla (puede fallar con una probabilidad aleatoria) se envía confirmar sino se envía abortar. Si se logra entregar el pedido se realiza el cobro efectivo, sino se cancelaría.

![Diagrama de secuencia](img/diagrams/secuencia-gateway.jpeg)

## Supuestos
- Se define la cantidad de instancias de interfaces en 3.
- La cantidad de instancias de robots será 5.
- La aplicación del gateway de pagos nunca se cae.
- En el caso de que se esté preparando un helado y no haya más stock del gusto a servir (recurso a consumir), se desecha todo lo servido previamente y el pedido queda cancelado.
- Cada **pedido** posee los siguientes atributos:
  - **id**: clave numérica única para cada uno.
  - **cliente**: datos de quien lo realiza.
  - **items**: lista de productos que lo conforman.
- Cada **cliente** cuenta con los siguientes atributos:
  - **id**: clave numérica única para cada uno.
  - **tarjeta de crédito**: los 16 números de la misma en formato string.
- Cada **producto** tiene los siguientes atributos:
  - **tipo**: puede ser vasito, cucurucho, 1/4 kg, 1/2 kg o 1 kg. 
  - **cantidad**: número de unidades del mismo.
  - **sabores**: lista de sabores que pueden ser chocolate, frutilla, vainilla, menta y limón. El máximo de sabores para cualquier producto es 3.
<!-- TODO:
  - Definir que ocurriria en el caso de que se caiga un robot mientras esta preparando un pedido, podria cancelarse o pasarse a otro robot. 
  - Definir que ocurriria en el caso de que se caiga una interfaz. -->
![Modelos de dominio](img/diagrams/gridrust.drawio.png)
## Conclusión

