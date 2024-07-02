<div align="center">
  <img src="img/grido.png" width="45%">
&nbsp; &nbsp; &nbsp; &nbsp;
  <img src="img/rust_crab.png" width="45%">

  # GridRust
</div>

[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-24ddc0f5d75046c5622901739e7c5dd533143b0c8e959d652212380cedb1ea36.svg)](https://classroom.github.com/a/PKo6zpFB)

## Índice
- [GridRust](#gridrust)
  - [Índice](#índice)
  - [Diseño](#diseño)
    - [Interfaces de Clientes](#interfaces-de-clientes)
      - [Resiliencia en las pantallas](#resiliencia-en-las-pantallas)
    - [Gestión de Pedidos](#gestión-de-pedidos)
      - [Resiliencia en los robots](#resiliencia-en-los-robots)
    - [Gateway de Pagos](#gateway-de-pagos)
  - [Comunicación entre procesos](#comunicación-entre-procesos)
    - [Protocolos de mensajes](#protocolos-de-mensajes)
      - [Mensajes de Interfaces de Clientes a Gestión de Pedidos y a Gateway de Pagos](#mensajes-de-interfaces-de-clientes-a-gestión-de-pedidos-y-a-gateway-de-pagos)
      - [Mensajes de Gestión de Pedidos y Gateway de Pagos a Interfaces de Clientes](#mensajes-de-gestión-de-pedidos-y-gateway-de-pagos-a-interfaces-de-clientes)
  - [Modelo de dominio](#modelo-de-dominio)
  - [Supuestos](#supuestos)

## Diseño
Se tienen tres aplicaciones distintas que se comunican a través de sockets UDP:
- **Interfaces de Clientes**: Modela las pantallas con las que los clientes hacen sus pedidos.
- **Gestión de Pedidos**: Simula los robots que preparan los helados. 
- **Gateway de Pagos**: Es donde se captura y luego efectiviza el pago. 

![Diagrama del Proyecto](img/diagrams/C4_gridrust.drawio.png)

### Interfaces de Clientes
Se modela cada interfaz de cliente o pantalla, la cual lee de un archivo pedidos simulados y los convierte en **órdenes de pedidos**. Luego, inicia una transacción por cada uno. En este caso, es un **pedido de helado**. Se plantea utilizar programación asincrónica para esperar por la respuesta y, mientras tanto, dar oportunidad a atender otro pedido. Se generan varias instancias (como distintos procesos) de esta aplicación para simular un número constante de pantallas en la heladería. Para llevar a cabo la transacción se plantea utilizar:
- **Commit de dos fases**: Cada instancia actúa como **coordinador** del pedido que esté procesando. En este caso el compromiso es entregar el helado solicitado. Los pasos del algoritmo son:
  1. El coordinador que ejecuta la orden de pedido escribe en su log _prepare_ indicando que inicia la preparación del pedido y le envía a Gateway de Pagos el mensaje _prepare_, para preguntar si puede capturar el pago.
  2. En el caso de pago capturado satisfactoriamente, envía _prepare_ a Gestión de Pedidos. De lo contrario aborta la transacción.
  3. Si el pedido es preparado correctamente, el coordinador efectúa y finaliza el compromiso enviando un mensaje _commit_ al robot y al Gateway de Pagos para efectivizar el cobro. Caso contrario se aborta el pedido y se cancela el pago.

#### Resiliencia en las pantallas

- Para verificar el estado de cada pantalla, se envían mensajes tipo _ping_ entre sí cada cierto tiempo para verificar que siguen procesando pedidos. En el mensaje _ping_ se envía información del último pedido completado. De esta forma, una pantalla puede tomar los pedidos de esa pantalla caída desde esa orden. Se utilizaría el modelo de actores para la comunicación entre las pantallas.
- Cuando se detecta que una pantalla está caída, los pedidos que estaba manejando se reasignan a otra pantalla. Ya se tendría establecido qué pantalla se hace cargo de qué pantalla en caso de que se caiga alguna. Por ejemplo: tenemos las pantallas 0, 1, 2, 3. Si se cae la 0, se hace cargo la 1, si se cae la 1, se hace cargo la 2, si se cae la 3, se hace cargo la 0.

### Gestión de Pedidos
 Esta aplicación se comunica con **Interfaces de Clientes**, recibiendo órdenes de pedidos y respondiendo si el robot pudo preparar el pedido para su entrega. Se plantea utilizar las siguientes herramientas de concurrencia:
- **Modelo de actores** para los robots:
Tienen como estado interno el contenedor que están empleando, en caso de que estén usando alguno. Los tipos de mensajes serán para solicitar un contenedor, liberarlo, y para otorgar o denegar su acceso. 
- **Algoritmo Centralizado** para sincronizar los accesos a los contenedores de helado por parte de los robots con optimización:
  - Se elige a un robot como coordinador.
  - Cada robot le envía al coordinador un vector con los contenedores (sabores) a los que necesita acceder.
  - El coordinador recorre el vector y le da acceso al primer contenedor que esté disponible.
  - Si hay algún contenedor disponible, le envía un enum Response::AccesoConcedido(IceCreamFlavor).
  - Si ningún contenedor está disponible, le manda un enum Response::AccesoDenegado(<razón>) con la razón por la cuál no pudo acceder. Además, agrega la request del robot a una cola.
  - Cuando se libera algún contenedor, el coordinador saca la/s request/s de la cola y se fija si el contenedor que se liberó le sirve a algún robot.
  
  Se decidió utilizar este algoritmo, porque, tal como se indica en el libro _Distributed Operating Systems_ de Tanenbaum, es el más simple de los algoritmos. Citando el libro, "El algoritmo centralizado es el más sencillo y también el más eficiente. Sólo requiere de tres mensajes para entrar y salir de una región critica: una solicitud y otorgamiento para entrar y una liberación para salir". El único problema que puede ocurrir es que falle el coordinador, pero existen algoritmos para detectar esto y elegir otro.
- **Algoritmo Bully** para elegir robot coordinador al inicio y en caso de que falle (cuando un robot observa que el coordinador ya no responde las solicitudes por un timeout definido), inicia una elección:
  1. El robot envía _ELECTION_ a los demás procesos con un id mayor.
  2. Si nadie responde, este gana la elección y se convierte en el coordinador. Se anuncia enviando un mensaje _COORDINATOR_ a todo el resto.
  3. Si alguno de los robots con id mayor le responde _OK_, este repite el mismo proceso y el trabajo del robot que llamó a elecciones termina.

  En el caso en que un robot estaba esperando para entrar en la sección crítica cuando cambia el coordinador, cuando termina la elección del nuevo coordinador, el robot que estaba esperando vuelve a solicitar el acceso al nuevo coordinador.
  
  Por lo visto en la bibliografía, no hay mucha diferencia entre los algoritmos de elección, no hay ventajas significativas entre elegir uno u otro.

#### Resiliencia en los robots

- Para verificar el estado de cada robot, el coordinador enviará un mensaje _ping_ a cada uno de ellos. Si no se recibe respuesta en un tiempo determinado, se considerará que el robot está caído.
- Cuando se detecta que un robot está caído y estaba procesando un pedido, el coordinador reasigna el pedido a otro robot. Para poder hacer esto, el coordinador mantiene un diccionario con lo que está haciendo cada robot. 
- Cuando se cambia el coordinador, cada robot le manda al coordinador el pedido que estaba haciendo, junto con la pantalla que lo pidió. Luego, el coordinador nuevo le envía a cada pantalla de nuevo _ready_ para el pedido que pidió y se está haciendo.

### Gateway de Pagos
Será una aplicación simple que _loguea_, tal como indica el enunciado, en un archivo. Se tendrá una sola instancia de la misma que se encargará de recibir mensajes _prepare_  del coordinador (que se encuentra en **Interfaces de Clientes**), preguntando si se puede capturar el pago (la tarjeta puede fallar con una probabilidad aleatoria). Su respuesta será _ready_ o _abort_ dependiendo el caso. Luego, si se logra entregar el pedido correctamente, se recibirá un mensaje _commit_ y se realizará el cobro efectivo.

## Comunicación entre procesos
Para asegurar una comunicación confiable entre los procesos usando sockets UDP, cada mensaje enviado esperará una respuesta del receptor. En caso de no recibir respuesta en un tiempo determinado, se considerará que se perdió el paquete y se reenviará el mensaje. Se utilizará un protocolo de comunicación simple, donde cada mensaje tendrá un formato específico.

A continuación se presentan diagramas de secuencia que muestran el intercambio de mensajes entre las entidades en distintos escenarios:

- Pedido realizado correctamente
  
![Secuencia feliz](img/diagrams/happy_sequence.png)

- Pedidos cancelados por captura del pago rechazada y por falta de stock de algún sabor
  
![Secuencia abort](img/diagrams/abort_sequences.png)

### Protocolos de mensajes
#### Mensajes de Interfaces de Clientes a Gestión de Pedidos y a Gateway de Pagos
Las pantallas enviarán tanto a Gestión de Pedidos como a Gateway de Pagos mensajes con el siguiente formato:

					{mensaje}\n{payload}\0
     
Donde el payload es la orden serializada en formato JSON:
```
pub struct Order {  
  order_id: usize,  
  client_id: usize,  
  credit_card: String,  
  items: Vec<Item>  
}
```

El mensaje puede ser de tres tipos:
- Prepare: Se envía al principio a Gestión de Pedidos y a Gateway de Pagos.
- Commit: Cuando se le entrega al cliente el helado se le envía a ambas aplicaciones.
- Abort: Se envía en caso de que falle alguna de las partes de la transacción.
Por lo tanto, se mantiene esta estructura a nivel global entre las tres aplicaciones

### Mensaje entre Pantallas
Las pantallas enviarán mensajes tipo Ping a la pantalla que tengan a cargo para verificar si sigue vigente:

              {mensaje}\n{payload}

Donde el mensaje puede ser de tipo *screen* y el payload es un enum llamado *ScreenMessage* que puede ser de tipo:
- Ping: lo envía una pantalla para vertificar si las pantalla sigue activa.
- Pong: Es la respuesta de la pantalla con el id de la última orden procesada.

#### Mensajes de Gestión de Pedidos y Gateway de Pagos a Interfaces de Clientes
Tanto el Gateway de Pagos como Gestión de Pedidos utilizarán el siguiente formato para el envío de mensajes:
			
   					{message}\n{payload}\0
El payload es la _Order_ serializada en formato JSON.
El mensaje podrá ser de tipo:
- Ready: indica que se pudo realizar ya sea el pedido o la captura del pago.
- Abort: indica que no se pudo preparar el pedido o que falló la tarjeta de crédito del cliente.
- Finished: es la respuesta que se le da al cliente cuando se llega a la segunda fase de la transacción, es la respuesta al mensaje de **Commit**.
- Keepalive: indica que se está intentando terminar el pedido de helado o la captura.

### Cómo ejecutar las aplicaciones

#### Gestión de Pedidos

Se corre cada robot con:
cargo run --bin robot n

Al coordinador con:
cargo run --bin coordinador

#### Interfaces de Clientes

Se ejecutan las pantallas con
cargo run --bin clients_interfaces

#### Mensajes entre Robots y Coordinador
Para pedir y liberar el acceso a los contenedores de helado e indicarle al coordinador que se completó la orden, se utilizará el siguiente formate de mensaje: 
			
   					{access}\n{payload}\0

El payload es un tipo de enum `RequestToCoordinator` serializado en formato JSON.
El payload puede ser: 
- SolicitarAcceso: incluye el id del robot y el vector de sabores a los que se pide acceso.
- LiberarAcceso: incluye el id del robot y el sabor de helado al que tenía acceso.
- OrdenTerminada: incluye el id del robot y la _Order_ completada.

El coordinar para contestarle a los robots y asignar pedidos, utiliza el siguiente formato de mensaje: 

   					{access}\n{payload}\0

El payload es un tipo de enum `Response` serializado en formato JSON.
El payload puede ser: 
- AccesoConcedido: incluye el sabor de helado al que le dió acceso.
- AccesoDenegado: incluye la razón por la cual no le pudo dar acceso.
- AssignOrder: incluye el id del robot y la _Order_ asignada.

## Modelo de dominio

 ![Modelos de dominio](img/diagrams/gridrust.drawio.png)

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

## Supuestos
- Se define la cantidad de instancias de interfaces de clientes en 3.
- La cantidad de instancias de robots será 5.
- La aplicación del Gateway de Pagos nunca se cae.
- En el caso de que un robot esté preparando un helado y no haya más stock del gusto a servir, se desecha todo lo servido previamente y el pedido queda cancelado.
- Los puertos de las pantallas y los robots son conocidos. 

