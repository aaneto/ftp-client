FROM python:3.8

WORKDIR /app
RUN mkdir res

COPY cat.png res/cat.png
COPY . .

RUN pip install --no-cache-dir -r requirements.txt

# Passive ports
EXPOSE 2558 2559

EXPOSE 20 21

CMD [ "python", "server.py" ]