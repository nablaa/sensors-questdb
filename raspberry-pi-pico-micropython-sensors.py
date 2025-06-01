from machine import Pin
from utime import sleep
import onewire
import ds18x20
import time
import network
import urequests

print("Starting program")

poll_interval_seconds = 60
database_host = "192.168.0.205"
database_port = 9000
table_name = "measurements"
location = "bedroom"

ssid = "XXX"
password = "XXX"
led_pin = Pin("LED", Pin.OUT)
ds_pin = Pin(22)
ds_sensor = ds18x20.DS18X20(onewire.OneWire(ds_pin))

print("Scanning sensors ...")
roms = ds_sensor.scan()
print("Found DS devices: ", roms)

print("Connecting to wlan ...")
wlan = network.WLAN(network.STA_IF)
wlan.active(True)
wlan.connect(ssid, password)

max_wait = 10
while max_wait > 0:
    if wlan.status() < 0 or wlan.status() >= 3:
        break
    max_wait -= 1
    print('waiting for connection ...')
    time.sleep(1)

if wlan.status() != 3:
    raise RuntimeError('network connection failed')
else:
    print('connected')
    status = wlan.ifconfig()
    print( 'ip = ' + status[0] )

def read_temperature():
    # Initialize the sensor
    ds_sensor.convert_temp()
    # Need to wait at least 750 ms before reading the temperature
    time.sleep_ms(750)
    for rom in roms:
        temperature = ds_sensor.read_temp(rom)
        return temperature

URL_RFC_3986 = {
"!": "%21", "#": "%23", "$": "%24", "&": "%26", "'": "%27", "(": "%28", ")": "%29", "*": "%2A", "+": "%2B", 
",": "%2C", "/": "%2F", ":": "%3A", ";": "%3B", "=": "%3D", "?": "%3F", "@": "%40", "[": "%5B", "]": "%5D",
' ': '%20',
}

def url_encoder(b):
    if type(b)==bytes:
        b = b.decode(encoding="utf-8") #byte can't insert many utf8 charaters
    result = bytearray() #bytearray: rw, bytes: read-only
    for i in b:
        if i in URL_RFC_3986:
            for j in URL_RFC_3986[i]:
                result.append(ord(j))
            continue
        i = bytes(i, "utf-8")
        if len(i)==1:
            result.append(ord(i))
        else:
            for c in i:
                c = hex(c)[2:].upper()
                result.append(ord("%"))
                result.append(ord(c[0:1]))
                result.append(ord(c[1:2]))
    result = result.decode("ascii")
    return result

def http_get(url):
    import socket
    _, _, host, path = url.split('/', 3)
    hostname, port = host.split(':', 2)
    addr = socket.getaddrinfo(hostname, int(port))[0][-1]
    s = socket.socket()
    s.connect(addr)
    s.send(bytes('GET /%s HTTP/1.0\r\nHost: %s\r\n\r\n' % (path, host), 'utf8'))
    data = s.recv(256)
    print(str(data, 'utf8'))
    s.close()

def send_reading(temperature):
    sql_query = "INSERT INTO {} (location, temperature, humidity, pressure, timestamp) VALUES ('{}', {}, null, null, now());".format(table_name, location, temperature)
    url = "http://" + database_host + ":" + str(database_port) + "/exec?fmt=json&query=" + url_encoder(sql_query)
    http_get(url);

while True:
    try:
        print("Reading temperature ...")
        led_pin.on()
        temperature = read_temperature()
        print(temperature)
        print("Sending measurement ...")
        send_reading(temperature)
        print("Done.")
        led_pin.off()
        time.sleep(poll_interval_seconds)
    except:
        pass

print("Program ended")
