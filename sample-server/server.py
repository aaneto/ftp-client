from pyftpdlib.authorizers import DummyAuthorizer
from pyftpdlib.handlers import FTPHandler
from pyftpdlib.servers import FTPServer

authorizer = DummyAuthorizer()
authorizer.add_user("user", "user", "res/", perm="elradfmwMT")
authorizer.add_anonymous("res")

handler = FTPHandler
handler.authorizer = authorizer
handler.passive_ports = range(2558, 2560)

server = FTPServer(("0.0.0.0", 21), handler)
server.serve_forever()