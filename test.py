import asyncio

import httpx

from sunsetRollercoaster.component.mvdis import car_check


async def main():
    # r = await car_check.CarCheck().client.get('https://www.solarbus.com.tw')
    # resp = await car_check.CarCheck().check_car(queryType='2',companyNo='80639635',plateNo='KAB-3555')
    # print(r)
    # resp = await car_check.CarCheck().check_car(queryType='1',birthday='0670319',idNo='Q122548647')
    for i in range(1, 5):
        resp = await car_check.CarCheck().check_car(
            queryType="2", companyNo="80639635", plateNo="KAB-3555"
        )
        print(resp)

asyncio.run(
   main()
)
