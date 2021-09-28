# ReMUD Documentation

## Deploying to GitHub Pages

1. cd docs
2. rm -rf ./public
3. git clone https://github.com/siler/remud.git --branch gh-pages ./public
4. hugo
5. cd ./public
6. git add --all
7. git commit -m "Publishing to GitHub Pages"
8. git push
