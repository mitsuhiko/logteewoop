import time
import random
import socket
import string

stream = '65d9f237-2801-4b99-844f-cd9f29084b6e'

print 'stream', stream
s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
s.connect(('127.0.0.1', 8001))
s.send('POST /%s/write HTTP/1.0\r\nTransfer-Encoding: chunked\r\n\r\n' % stream)
for x in xrange(2000):
    data = '%s\r\n' % ''.join(random.choice(string.letters) for _ in xrange(random.randrange(5, 100)))
    s.send('%x\r\n%s\r\n' % (len(data), data))
    time.sleep(0.1)
s.send('0\r\n\r\n')
s.close()
