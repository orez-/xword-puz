# this is script is bad. when it fails it may leave you in any number of Weird States.
# do as i say, not as i do.
set -euxo pipefail
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "${SCRIPT_DIR}/.."

./scripts/build.sh
cp -R www target/www
git switch gh-pages

# Replace project root with contents of `www`
shopt -s extglob
mkdir tmp
mv !(tmp) tmp
mv tmp/target/ target/
mv target/www/* .
rm -r target/www
rm -rf tmp

git add .
date=`date '+%F %H:%M:%S'`
git commit -m "Publish $date" || $(exit 0)
git push
git switch -
