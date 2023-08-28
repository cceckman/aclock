#! /bin/sh
#

cd "$(dirname $0)"

if test -x env/bin/activate
then
  source env/bin/activate
fi

python3 aclock.py

