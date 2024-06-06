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
El modelo propuesto consta de tres aplicaciones distintas, las cuáles tendrán múltiples instancias que se comunicarán a través de sockets TCP.

![Diagrama del Proyecto](img/diagrams/C4_gridrust.drawio.png)

### Interfaces de clientes
Simulan las "pantallas" en las cuales los clientes hacen sus pedidos. Su procedimiento será el siguiente:
1. Procesamiento de los pedidos utilizando programación asincrónica ya que los mismos se obtendrán de un archivo *jsonl*. 
2. Intentar capturar el pago comunicándose con el gateway de pagos.
3. En el caso de recibir una respuesta positiva, comunicarse con un robot para que prepare el pedido.
4. Al recibir la respuesta del robot, en caso de haber podido realizar el pedido correctamente se realiza el cobro efectivo comunicándose nuevamente con el gateway de pagos.

### Gestión de pedidos con robots
- **Modelo de actores** para los robots:
tendrán como estado interno el contenedor que están usando o ninguno en el caso contrario. Los tipos de mensajes serán para solicitar, liberar, otorgar y denegar el acceso a un contenedor. También tendrán mensajes para iniciar una elección, para responder _OK_, y para avisar que fue elegido coordinador.
- **Algoritmo centralizado** para sincronizar los accesos a los contenedores de helado por parte de los robots:
Se elige a un robot como coordinador. Si un robot quiere usar alguno de los contenedores de helado le envía un mensaje de solicitud al coordinador, donde indica qué contenedor quiere usar y si ningún otro robot lo está usando el coordinador le responde _OK_ y lo deja entrar. En cambio, si ya hay algún robot utilizando ese contenedor el coordinador le envía _ACK_ y se bloquea el solicitante, agregando su solicitud a una cola. Cuando el robot que estaba usando el contenedor termina le avisa al coordinador y este saca al solicitante de la cola y para otorgarle el acceso al contenedor enviándole _OK_.
	
  Justificación: se cita el libro de Distributed Operating Systems de Tanenbaum: "El algoritmo centralizado es el más sencillo y también el más eficiente. Sólo requiere de tres mensajes para entrar y salir de una región critica: una solicitud y otorgamiento para entrar y una liberación para salir". El único problema que puede ocurrir es que falle el coordinador, pero existen algoritmos para detectar esto y elegir otro.

- **Algoritmo Bully** para elegir robot coordinador al inicio y en caso de que falle (cuando un robot observa que el coordinador ya no responde las solicitudes por un timeout que se define), inicia una elección:
  1. El robot envía _ELECTION_ a los demás procesos con un número mayor.
  2. Si nadie responde, este gana la elección y se convierte en el coordinador.
  3. Si uno de los robots con un número mayor responde, toma el control y el trabajo del robot que llamó a elecciones termina.

  Por lo visto en la bibliografía no hay mucha diferencia entre los algoritmos de elección, no hay ventajas significativas entre elegir uno u otro.

### Gateway de pagos
- **Commit de dos fases** para la captura (cuando se toma el pedido) y el pago efectivo (al momento de la entrega del pedido). En este caso el compromiso es entregar el helado solicitado. El proceso es el siguiente:
   1. Hay un proceso coordinador que ejecuta la transacción, este escribe en su log _prepare_ indicando que inicia la preparación del pedido y le envía al resto de los procesos _prepare_, para avisarles que estén listos para el compromiso.
  2. Cuando un proceso recibe el mensaje verifica si está listo para el compromiso, lo escribe en su log y envía su decisión.
  3. Si el coordinador recibe todas las respuestas de los procesos diciendo que están listos para comprometerse, se efectúa y finaliza el compromiso. Si alguno no se puede comprometer se     aborta la preparación del pedido.
    
Tanto al momento de la captura como del cobro efectivo, se loguean los datos del pedido junto con su estado en un _txt_.

En este caso aún resta definir si los procesos deberían ser los propios robots u otra estructura, podrían ser los contenedores de helado que se consultan para saber si hay suficiente de cada gusto para completar el pedido.

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

## Conclusión

