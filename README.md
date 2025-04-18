<!-- Improved compatibility of back to top link: See: https://github.com/othneildrew/Best-README-Template/pull/73 -->
<a id="readme-top"></a>
<!--
*** Thanks for checking out the Best-README-Template. If you have a suggestion
*** that would make this better, please fork the repo and create a pull request
*** or simply open an issue with the tag "enhancement".
*** Don't forget to give the project a star!
*** Thanks again! Now go create something AMAZING! :D
-->



<!-- PROJECT SHIELDS -->
<!--
*** I'm using markdown "reference style" links for readability.
*** Reference links are enclosed in brackets [ ] instead of parentheses ( ).
*** See the bottom of this document for the declaration of the reference variables
*** for contributors-url, forks-url, etc. This is an optional, concise syntax you may use.
*** https://www.markdownguide.org/basic-syntax/#reference-style-links
-->
[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![Unlicense License][license-shield]][license-url]



<!-- PROJECT LOGO -->
<br />
<div align="center">
  <h3 align="center">downloader-rs</h3>

  <p align="center">
    An unnecessarily complicated downloader
    <br />
    <br />
    <a href="https://github.com/casab/downloader-rs/issues/new?labels=bug&template=bug-report---.md">Report Bug</a>
    ·
    <a href="https://github.com/casab/downloader-rs/issues/new?labels=enhancement&template=feature-request---.md">Request Feature</a>
  </p>
</div>



<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
  </ol>
</details>



<!-- ABOUT THE PROJECT -->
## About The Project

I just wanted to use Rust for no particular reason. Therefore I decided to create this project.

<p align="right">(<a href="#readme-top">back to top</a>)</p>



### Built With

* [![Rust][Rust-lang]][Rust-url]
* [Tokio][Tokio-url]
* [Actix][Actix-url]


<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- GETTING STARTED -->
## Getting Started

To get a local copy up and running follow these simple example steps.

1. Run ```git clone https://github.com/casab/downloader-rs```
2. ```cd downloader-rs```
3. Run ```cargo run```

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- ROADMAP -->
## Roadmap

More like todo...

- [x] Add s3 compatible save target
- [x] Add users
- [ ] Add JWT authentication
- [ ] Add jobs for downloading by creating by producing events on api calls, and move actual downloading to consumers
- [ ] Collect metrics on downloading jobs by emitting event data for downloads


See the [open issues](https://github.com/casab/downloader-rs/issues) for a full list of proposed features (and known issues).

<p align="right">(<a href="#readme-top">back to top</a>)</p>


<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE.txt` for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- CONTACT -->
## Contact

Engin Al - [@etikmaske](https://twitter.com/etikmaske) - enginal@gmail.com

Project Link: [https://github.com/casab/downloader-rs](https://github.com/casab/downloader-rs)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/casab/downloader-rs.svg?style=for-the-badge
[contributors-url]: https://github.com/casab/downloader-rs/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/casab/downloader-rs.svg?style=for-the-badge
[forks-url]: https://github.com/casab/downloader-rs/network/members
[stars-shield]: https://img.shields.io/github/stars/casab/downloader-rs.svg?style=for-the-badge
[stars-url]: https://github.com/casab/downloader-rs/stargazers
[issues-shield]: https://img.shields.io/github/issues/casab/downloader-rs.svg?style=for-the-badge
[issues-url]: https://github.com/casab/downloader-rs/issues
[license-shield]: https://img.shields.io/github/license/casab/downloader-rs.svg?style=for-the-badge
[license-url]: https://github.com/casab/downloader-rs/blob/main/LICENSE.txt

[Rust-lang]: https://img.shields.io/badge/rust-000000?style=for-the-badge&logo=rust&logoColor=white
[Rust-url]: https://rust-lang.org/
[Tokio-url]: https://tokio.rs/
[Actix-url]: https://actix.rs/
